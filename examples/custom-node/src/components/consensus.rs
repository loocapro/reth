use std::sync::Arc;

use reth::{
    beacon_consensus::EthBeaconConsensus,
    builder::{components::ConsensusBuilder, BuilderContext},
    consensus::{ConsensusError, FullConsensus},
};
use reth_chainspec::ChainSpec;
use reth_node_api::{FullNodeTypes, NodeTypes};

use crate::types::primitives::CustomPrimitives;

/// A basic ethereum consensus builder.
#[derive(Debug, Default, Clone, Copy)]
pub struct CustomConsensusBuilder;

impl<Node> ConsensusBuilder<Node> for CustomConsensusBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec, Primitives = CustomPrimitives>>,
{
    type Consensus = Arc<dyn FullConsensus<CustomPrimitives, Error = ConsensusError>>;

    async fn build_consensus(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::Consensus> {
        Ok(Arc::new(EthBeaconConsensus::new(ctx.chain_spec())))
    }
}
