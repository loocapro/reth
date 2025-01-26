use reth_node_api::NodePrimitives;

/// Temp helper struct for integrating [`NodePrimitives`].
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct CustomPrimitives;

impl NodePrimitives for CustomPrimitives {
    type Block = reth::primitives::Block;
    type BlockHeader = alloy_consensus::Header;
    type BlockBody = reth::primitives::BlockBody;
    type SignedTx = reth::primitives::TransactionSigned;
    type Receipt = reth::primitives::Receipt;
}
