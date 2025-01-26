use reth::{
    builder::{components::NetworkBuilder, BuilderContext},
    network::{EthNetworkPrimitives, NetworkHandle, PeersInfo},
    primitives::PooledTransaction,
};
use reth_chainspec::ChainSpec;
use reth_node_api::{FullNodeTypes, NodeTypes, TxTy};
use reth_tracing::tracing::info;
use reth_transaction_pool::{PoolTransaction, TransactionPool};

use crate::types::primitives::CustomPrimitives;

#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct CustomNetworkBuilder;

impl<Node, Pool> NetworkBuilder<Node, Pool> for CustomNetworkBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec, Primitives = CustomPrimitives>>,
    Pool: TransactionPool<
            Transaction: PoolTransaction<Consensus = TxTy<Node::Types>, Pooled = PooledTransaction>,
        > + Unpin
        + 'static,
{
    type Primitives = EthNetworkPrimitives;

    async fn build_network(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<NetworkHandle> {
        let network = ctx.network_builder().await?;
        let handle = ctx.start_network(network, pool);
        info!(target: "reth::cli", enode=%handle.local_node_record(), "P2P networking initialized");
        Ok(handle)
    }
}
