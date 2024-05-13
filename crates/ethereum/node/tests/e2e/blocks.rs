use reth::{
    rpc::{
        api::EngineApiClient,
        types::{
            engine::PayloadStatusEnum, ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3,
        },
    },
    tasks::TaskManager,
};
use reth_e2e_test_utils::TestNodeGenerator;
use serde_json::{from_reader, to_string_pretty};
use std::{
    fs::{create_dir_all, remove_dir_all, File},
    io::{BufReader, Write},
    sync::Arc,
};

use reth_node_ethereum::{EthEngineTypes, EthereumNode};
use reth_primitives::{ChainSpecBuilder, Genesis, SealedBlock, MAINNET};

use crate::utils::eth_payload_attributes;

#[tokio::test]
async fn can_replay_blocks() -> eyre::Result<()> {
    reth_tracing::init_test_tracing();

    let tasks = TaskManager::current();
    let exec = tasks.executor();

    let genesis: Genesis = serde_json::from_str(include_str!("../assets/genesis.json")).unwrap();
    let chain_spec = Arc::new(
        ChainSpecBuilder::default()
            .chain(MAINNET.chain)
            .genesis(genesis)
            .cancun_activated()
            .build(),
    );

    // advance first node to 10
    let mut node =
        TestNodeGenerator::<EthereumNode>::new(chain_spec.clone(), exec.clone()).gen().await?;
    let last_block = 10;
    let canonical_chain = node.advance_many(last_block, eth_payload_attributes).await.unwrap();

    // collect blocks and attributes
    let blocks =
        canonical_chain.iter().map(|(payload, _)| (payload.block().clone())).collect::<Vec<_>>();
    let last_block_hash = blocks.last().unwrap().hash();
    let attrs = canonical_chain.iter().map(|(_, att)| att.clone()).collect::<Vec<_>>();

    // write and read blocks from file
    let block_test_ctx = BlockTestContext::new(blocks, "./tmp");
    let blocks = block_test_ctx.write_and_read_blocks()?;
    // from sealed blocks to payload v3
    let payloads = block_test_ctx.into_exec_payload_v3(blocks);

    // replay blocks
    let node = TestNodeGenerator::<EthereumNode>::new(chain_spec, exec).gen().await?;
    for (i, p) in payloads.iter().enumerate() {
        let submission = EngineApiClient::<EthEngineTypes>::new_payload_v3(
            &node.engine_api.engine_api_client,
            p.clone(),
            vec![],
            attrs[i].parent_beacon_block_root.unwrap(),
        )
        .await?;

        assert_eq!(submission.status, PayloadStatusEnum::Valid);

        let latest_hash = submission.latest_valid_hash.unwrap_or_default();

        node.engine_api.update_forkchoice(latest_hash, latest_hash).await?;
    }

    // make sure all the blocks are replayed
    node.assert_new_block(last_block_hash, last_block).await?;
    Ok(())
}

#[derive(Debug)]
struct BlockTestContext {
    dir_path: String,
    file_path: String,
    blocks: Vec<SealedBlock>,
}

impl BlockTestContext {
    fn new(blocks: Vec<SealedBlock>, dir_path: &str) -> Self {
        let file_path = format!("{}/blocks.json", dir_path);
        Self { dir_path: dir_path.to_string(), file_path, blocks }
    }
    fn write_and_read_blocks(&self) -> eyre::Result<Vec<SealedBlock>> {
        self.write_to_file()?;
        self.read_from_file()
    }

    fn write_to_file(&self) -> eyre::Result<()> {
        create_dir_all(&self.dir_path)?;
        let json = to_string_pretty(&self.blocks)?;
        let mut file = File::create(&self.file_path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    fn read_from_file(&self) -> eyre::Result<Vec<SealedBlock>> {
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let blocks: Vec<SealedBlock> = from_reader(reader)?;
        remove_dir_all(&self.dir_path)?;
        Ok(blocks)
    }

    fn into_exec_payload_v3(self, blocks: Vec<SealedBlock>) -> Vec<ExecutionPayloadV3> {
        blocks.into_iter().map(BlockTestContext::convert_to_payload_v3).collect()
    }

    fn convert_to_payload_v3(block: SealedBlock) -> ExecutionPayloadV3 {
        ExecutionPayloadV3 {
            payload_inner: ExecutionPayloadV2 {
                payload_inner: ExecutionPayloadV1 {
                    parent_hash: block.header.parent_hash,
                    fee_recipient: block.header.beneficiary,
                    state_root: block.header.state_root,
                    receipts_root: block.header.receipts_root,
                    logs_bloom: block.header.logs_bloom,
                    prev_randao: block.header.mix_hash,
                    block_number: block.header.number,
                    gas_limit: block.header.gas_limit,
                    gas_used: block.header.gas_used,
                    timestamp: block.header.timestamp,
                    extra_data: block.header.extra_data.clone(),
                    base_fee_per_gas: block.header.base_fee_per_gas.unwrap().try_into().unwrap(),
                    block_hash: block.header.hash(),
                    transactions: block.raw_transactions(),
                },
                withdrawals: block.withdrawals.clone().unwrap_or_default().to_vec(),
            },
            blob_gas_used: block.header.blob_gas_used.unwrap(),
            excess_blob_gas: block.header.excess_blob_gas.unwrap(),
        }
    }
}
