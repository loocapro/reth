use reth::{
    builder::{
        components::ComponentsBuilder, rpc::RpcAddOns, Node, NodeAdapter, NodeComponentsBuilder,
    },
    network::NetworkHandle,
    providers::EthStorage,
    rpc::eth::EthApi,
};
use reth_chainspec::ChainSpec;
use reth_node_api::{FullNodeComponents, FullNodeTypes, NodeTypesWithEngine};

use crate::{
    components::{
        consensus::CustomConsensusBuilder, executor::CustomExecutorBuilder,
        network::CustomNetworkBuilder, pool::CustomPoolBuilder,
    },
    types::{
        engine::{
            payload_builder::CustomPayloadServiceBuilder, CustomEngineTypes,
            CustomEngineValidatorBuilder,
        },
        primitives::CustomPrimitives,
    },
};

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub(crate) struct MyCustomNode;

/// Implement the Node trait for the custom node
///
/// This provides a preset configuration for the node
impl<N> Node<N> for MyCustomNode
where
    N: FullNodeTypes<
        Types: NodeTypesWithEngine<
            Engine = CustomEngineTypes,
            ChainSpec = ChainSpec,
            Primitives = CustomPrimitives,
            Storage = EthStorage,
        >,
    >,
{
    type ComponentsBuilder = ComponentsBuilder<
        N,
        CustomPoolBuilder,
        CustomPayloadServiceBuilder,
        CustomNetworkBuilder,
        CustomExecutorBuilder,
        CustomConsensusBuilder,
    >;

    type AddOns = MyNodeAddOns<
        NodeAdapter<N, <Self::ComponentsBuilder as NodeComponentsBuilder<N>>::Components>,
    >;

    fn components_builder(&self) -> Self::ComponentsBuilder {
        ComponentsBuilder::default()
            .node_types::<N>()
            .pool(CustomPoolBuilder::default())
            .payload(CustomPayloadServiceBuilder::default())
            .network(CustomNetworkBuilder::default())
            .executor(CustomExecutorBuilder::default())
            .consensus(CustomConsensusBuilder::default())
    }

    fn add_ons(&self) -> Self::AddOns {
        MyNodeAddOns::default()
    }
}

pub type MyNodeAddOns<N> = RpcAddOns<
    N,
    EthApi<
        <N as FullNodeTypes>::Provider,
        <N as FullNodeComponents>::Pool,
        NetworkHandle,
        <N as FullNodeComponents>::Evm,
    >,
    CustomEngineValidatorBuilder,
>;
