use alloy_genesis::Genesis;
use node::MyCustomNode;
use reth::{builder::NodeBuilder, tasks::TaskManager};
use reth_chainspec::{Chain, ChainSpec};
use reth_node_core::{args::RpcServerArgs, node_config::NodeConfig};
use reth_tracing::{RethTracer, Tracer};

mod components;
mod node;
mod types;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let _guard = RethTracer::new().init()?;

    let tasks = TaskManager::current();

    let spec = ChainSpec::builder()
        .chain(Chain::mainnet())
        .genesis(Genesis::default())
        .london_activated()
        .paris_activated()
        .shanghai_activated()
        .build();

    let node_config =
        NodeConfig::test().with_rpc(RpcServerArgs::default().with_http()).with_chain(spec);

    let handle = NodeBuilder::new(node_config)
        .testing_node(tasks.executor())
        .launch_node(MyCustomNode::default())
        .await
        .unwrap();

    println!("Node started");

    handle.node_exit_future.await
}
