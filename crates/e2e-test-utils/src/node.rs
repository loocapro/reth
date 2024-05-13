use crate::{
    engine_api::EngineApiTestContext,
    network::NetworkTestContext,
    payload::PayloadTestContext,
    rpc::RpcTestContext,
    traits::PayloadEnvelopeExt,
    transaction::TransactionStream,
    wallet::{Wallet, WalletGenerator},
};
use alloy_rpc_types::BlockNumberOrTag;
use eyre::Ok;
use futures_util::{Future, StreamExt};
use reth::{
    api::{BuiltPayload, EngineTypes, FullNodeComponents, PayloadBuilderAttributes},
    builder::FullNode,
    providers::{BlockReader, BlockReaderIdExt, CanonStateSubscriptions, StageCheckpointReader},
    rpc::types::engine::PayloadStatusEnum,
};
use reth_node_builder::NodeTypes;
use reth_primitives::{stage::StageId, BlockHash, BlockNumber, Bytes, B256, MAINNET};
use std::{marker::PhantomData, sync::Arc};

/// An helper struct to handle node actions
pub struct NodeTestContext<Node>
where
    Node: FullNodeComponents,
{
    pub inner: FullNode<Node>,
    pub payload: PayloadTestContext<Node::Engine>,
    pub network: NetworkTestContext<Node>,
    pub engine_api: EngineApiTestContext<Node::Engine>,
    pub rpc: RpcTestContext<Node>,
    pub wallet: Wallet,
    pub tx_stream: Option<TransactionStream>,
}

impl<Node> NodeTestContext<Node>
where
    Node: FullNodeComponents,
{
    /// Creates a new test node
    pub async fn new(node: FullNode<Node>) -> eyre::Result<NodeTestContext<Node>> {
        let builder = node.payload_builder.clone();

        let wallet = WalletGenerator::new().chain_id(MAINNET.chain).gen();
        let wall_clone = wallet.clone();

        // default tx generator is eip1559
        let generator_fn = move || {
            let mut wallet = wallet.clone();
            Box::pin(async move { wallet.eip1559().await })
        };
        let tx_stream = TransactionStream::new(generator_fn);

        Ok(Self {
            inner: node.clone(),
            payload: PayloadTestContext::new(builder).await?,
            network: NetworkTestContext::new(node.network.clone()),
            engine_api: EngineApiTestContext {
                engine_api_client: node.auth_server_handle().http_client(),
                canonical_stream: node.provider.canonical_state_stream(),
                _marker: PhantomData::<Node::Engine>,
            },
            rpc: RpcTestContext { inner: node.rpc_registry },
            wallet: wall_clone,
            tx_stream: Some(tx_stream),
        })
    }

    /// Overrides the default tx generator with a given tx generator
    pub fn set_tx_generator<F, Fut>(mut self, tx_generator: Arc<F>) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Bytes> + Send + 'static,
    {
        self.tx_stream = Some(TransactionStream::new(move || (tx_generator)()));
        self
    }

    /// Inject the pending transactions from the tx stream into the node mempool
    pub fn inject_pending_stream(&mut self) {
        // Inject stream in the tx pool in background
        let stream = self.tx_stream.take().expect("TransactionStream is not set");
        self.rpc.inject_stream(stream);
    }

    /// Advances the chain `length` blocks.
    ///
    /// Returns the added chain as a Vec of block hashes and the payload attributes used to build.
    pub async fn advance_many(
        &mut self,
        length: u64,
        attributes_generator: impl Fn(u64) -> <Node::Engine as EngineTypes>::PayloadBuilderAttributes
            + Copy,
    ) -> eyre::Result<
        Vec<(
            <<Node as NodeTypes>::Engine as EngineTypes>::BuiltPayload,
            <Node::Engine as EngineTypes>::PayloadBuilderAttributes,
        )>,
    >
    where
        <Node::Engine as EngineTypes>::ExecutionPayloadV3:
            From<<Node::Engine as EngineTypes>::BuiltPayload> + PayloadEnvelopeExt,
    {
        let mut chain = Vec::with_capacity(length as usize);

        self.inject_pending_stream();

        for _ in 0..length {
            let (payload, attr, _) = self.advance(vec![], attributes_generator).await?;
            chain.push((payload, attr));
        }
        Ok(chain)
    }

    /// Advances the node forward one block
    pub async fn advance(
        &mut self,
        versioned_hashes: Vec<B256>,
        attributes_generator: impl Fn(u64) -> <Node::Engine as EngineTypes>::PayloadBuilderAttributes,
    ) -> eyre::Result<(
        <Node::Engine as EngineTypes>::BuiltPayload,
        <Node::Engine as EngineTypes>::PayloadBuilderAttributes,
        B256,
    )>
    where
        <Node::Engine as EngineTypes>::ExecutionPayloadV3:
            From<<Node::Engine as EngineTypes>::BuiltPayload> + PayloadEnvelopeExt,
    {
        let (payload, eth_attr) = self.new_payload(attributes_generator).await?;

        let block_hash = self
            .engine_api
            .submit_payload(
                payload.clone(),
                eth_attr.clone(),
                PayloadStatusEnum::Valid,
                versioned_hashes,
            )
            .await?;

        // trigger forkchoice update via engine api to commit the block to the blockchain
        self.engine_api.update_forkchoice(block_hash, block_hash).await?;

        // assert the block has been committed to the blockchain
        let block_hash = payload.block().hash();
        let block_number = payload.block().number;
        self.assert_new_block(block_hash, block_number).await?;

        Ok((payload, eth_attr, block_hash))
    }

    /// Creates a new payload from given attributes generator
    /// expects a payload attribute event and waits until the payload is built.
    ///
    /// It triggers the resolve payload via engine api and expects the built payload event.
    pub async fn new_payload(
        &mut self,
        attributes_generator: impl Fn(u64) -> <Node::Engine as EngineTypes>::PayloadBuilderAttributes,
    ) -> eyre::Result<(
        <<Node as NodeTypes>::Engine as EngineTypes>::BuiltPayload,
        <<Node as NodeTypes>::Engine as EngineTypes>::PayloadBuilderAttributes,
    )>
    where
        <Node::Engine as EngineTypes>::ExecutionPayloadV3:
            From<<Node::Engine as EngineTypes>::BuiltPayload> + PayloadEnvelopeExt,
    {
        // trigger new payload building draining the pool
        let eth_attr = self.payload.new_payload(attributes_generator).await.unwrap();
        // first event is the payload attributes
        self.payload.expect_attr_event(eth_attr.clone()).await?;
        // wait for the payload builder to have finished building
        self.payload.wait_for_built_payload(eth_attr.payload_id()).await;
        // trigger resolve payload via engine api
        self.engine_api.get_payload_v3_value(eth_attr.payload_id()).await?;
        // ensure we're also receiving the built payload as event
        Ok((self.payload.expect_built_payload().await?, eth_attr))
    }

    /// Waits for a block to be available on the node, ensuring it reaches the finish checkpoint.
    pub async fn wait_until_block_is_available(
        &self,
        number: BlockNumber,
        expected_block_hash: BlockHash,
    ) -> eyre::Result<()> {
        // Loop until the finish checkpoint is reached and the block matches the expected hash.
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;

            // Check if the finish checkpoint has been reached.
            if self.has_reached_finish_checkpoint(number).await? {
                // Attempt to fetch the block matching the expected hash.
                if let Some(latest_block) = self.inner.provider.block_by_number(number)? {
                    // Assert the block's hash matches the expected hash to proceed.
                    assert_eq!(latest_block.hash_slow(), expected_block_hash);
                    break;
                } else {
                    // Panic if the finish checkpoint matches but the block could not be fetched.
                    panic!("Finish checkpoint matches, but could not fetch block.");
                }
            }
        }
        Ok(())
    }

    /// Checks if the node has reached the finish checkpoint for the given block number.
    async fn has_reached_finish_checkpoint(&self, block_number: BlockNumber) -> eyre::Result<bool> {
        if let Some(checkpoint) = self.inner.provider.get_stage_checkpoint(StageId::Finish)? {
            return Ok(checkpoint.block_number >= block_number);
        }
        Ok(false)
    }

    pub async fn wait_unwind(&self, number: BlockNumber) -> eyre::Result<()> {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if let Some(checkpoint) = self.inner.provider.get_stage_checkpoint(StageId::Headers)? {
                if checkpoint.block_number == number {
                    break
                }
            }
        }
        Ok(())
    }

    pub async fn assery_tx_hash(&mut self, tip_tx_hash: B256) {
        // get head block from notifications stream and verify the tx has been pushed to the
        // pool is actually present in the canonical block
        let head = self.engine_api.canonical_stream.next().await.unwrap();
        let tx = head.tip().transactions().next();
        assert_eq!(tx.unwrap().hash().as_slice(), tip_tx_hash.as_slice());
    }

    pub async fn assert_new_block(
        &self,
        block_hash: B256,
        block_number: BlockNumber,
    ) -> eyre::Result<()> {
        loop {
            // wait for the block to commit
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            if let Some(latest_block) =
                self.inner.provider.block_by_number_or_tag(BlockNumberOrTag::Latest)?
            {
                if latest_block.number == block_number {
                    // make sure the block hash we submitted via FCU engine api is the new latest
                    // block using an RPC call
                    assert_eq!(latest_block.hash_slow(), block_hash);
                    break
                }
            }
        }
        Ok(())
    }
}
