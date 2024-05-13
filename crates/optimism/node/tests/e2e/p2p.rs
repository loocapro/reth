use crate::utils::optimism_payload_attributes;
use reth::{primitives::BASE_MAINNET, tasks::TaskManager};
use reth_e2e_test_utils::{wallet::WalletGenerator, TestNetworkBuilder};
use reth_interfaces::blockchain_tree::error::BlockchainTreeError;
use reth_node_optimism::OptimismNode;
use reth_primitives::{ChainSpecBuilder, Genesis};
use reth_rpc_types::engine::PayloadStatusEnum;
use std::sync::Arc;

#[tokio::test]
async fn can_sync() -> eyre::Result<()> {
    reth_tracing::init_test_tracing();

    let tasks = TaskManager::current();
    let exec = tasks.executor();

    let genesis: Genesis = serde_json::from_str(include_str!("../assets/genesis.json"))?;
    let chain_spec = Arc::new(
        ChainSpecBuilder::default()
            .chain(BASE_MAINNET.chain)
            .genesis(genesis)
            .ecotone_activated()
            .build(),
    );

    // setup wallets and generator functions
    let mut wallets = WalletGenerator::new().chain_id(BASE_MAINNET.chain).gen_many(2);

    let wallet = wallets.pop().unwrap();
    let generator_fn = move || {
        let mut wallet = wallet.clone();
        Box::pin(async move { wallet.optimism_block_info().await })
    };

    let second_wallet = wallets.pop().unwrap();
    let generator_fn_2 = move || {
        let mut wallet = second_wallet.clone();
        Box::pin(async move { wallet.optimism_block_info().await })
    };

    // setup 3 nodes
    let mut nodes = TestNetworkBuilder::<OptimismNode>::new(3, chain_spec, exec)
        .set_tx_generator(generator_fn)
        .build()
        .await?;
    let third_node = nodes.pop().unwrap();
    // override tx generator using a separate wallet
    let mut second_node = nodes.pop().unwrap().set_tx_generator(Arc::new(generator_fn_2));
    let mut first_node = nodes.pop().unwrap();

    // setup tip and reorg depth
    let tip: usize = 90;
    let tip_index: usize = tip - 1;
    let reorg_depth = 2;

    // On first node, create a chain up to block number 90a
    let canonical_payload_chain =
        first_node.advance_many(tip as u64, optimism_payload_attributes).await.unwrap();
    let canonical_chain =
        canonical_payload_chain.iter().map(|p| p.0.block().hash()).collect::<Vec<_>>();

    // On second node, sync optimistically up to block number 88a
    second_node
        .engine_api
        .update_optimistic_forkchoice(canonical_chain[tip_index - reorg_depth])
        .await?;
    second_node
        .wait_until_block_is_available(
            (tip - reorg_depth) as u64,
            canonical_chain[tip_index - reorg_depth],
        )
        .await?;

    // On third node, sync optimistically up to block number 90a
    third_node.engine_api.update_optimistic_forkchoice(canonical_chain[tip_index]).await?;
    third_node.wait_until_block_is_available(tip as u64, canonical_chain[tip_index]).await?;

    //  On second node, create a side chain: 88a -> 89b -> 90b
    second_node.payload.timestamp = first_node.payload.timestamp - reorg_depth as u64;
    let side_payload_chain =
        second_node.advance_many(reorg_depth as u64, optimism_payload_attributes).await.unwrap();
    let side_chain = side_payload_chain.iter().map(|p| p.0.block().hash()).collect::<Vec<_>>();
    // Creates fork chain by submitting 89b payload.
    // By returning Valid here, op-node will finally return a finalized hash
    third_node
        .engine_api
        .submit_payload(
            side_payload_chain[0].0.clone(),
            side_payload_chain[0].1.clone(),
            PayloadStatusEnum::Valid,
            Default::default(),
        )
        .await
        .unwrap();

    // It will issue a pipeline reorg to 88a, and then make 89b canonical AND finalized.
    third_node.engine_api.update_forkchoice(side_chain[0], side_chain[0]).await?;

    // Make sure we have the updated block
    third_node.wait_unwind((tip - reorg_depth) as u64).await?;
    third_node
        .wait_until_block_is_available(
            side_payload_chain[0].0.block().number,
            side_payload_chain[0].0.block().hash(),
        )
        .await?;

    // Make sure that trying to submit 89a again will result in an invalid payload status, since 89b
    // has been set as finalized.
    let _ = third_node
        .engine_api
        .submit_payload(
            canonical_payload_chain[tip_index - reorg_depth + 1].0.clone(),
            canonical_payload_chain[tip_index - reorg_depth + 1].1.clone(),
            PayloadStatusEnum::Invalid {
                validation_error: BlockchainTreeError::PendingBlockIsFinalized {
                    last_finalized: (tip - reorg_depth) as u64 + 1,
                }
                .to_string(),
            },
            Default::default(),
        )
        .await;

    Ok(())
}
