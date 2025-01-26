use alloy_rpc_types::engine::{
    ExecutionPayload, ExecutionPayloadEnvelopeV2, ExecutionPayloadEnvelopeV3,
    ExecutionPayloadEnvelopeV4, ExecutionPayloadSidecar, ExecutionPayloadV1,
    PayloadAttributes as EthPayloadAttributes,
};
use built_payload::CustomBuiltPayload;
use reth::{
    builder::{rpc::EngineValidatorBuilder, AddOnsContext},
    primitives::SealedBlock,
    rpc::compat::engine::payload::block_to_payload,
};
use reth_chainspec::ChainSpec;
use reth_node_api::{EngineTypes, FullNodeComponents, NodeTypesWithEngine, PayloadTypes};

use reth_node_ethereum::node::EthereumEngineValidator;
use reth_payload_builder::EthPayloadBuilderAttributes;
use serde::{Deserialize, Serialize};

use super::primitives::CustomPrimitives;

pub(crate) mod built_payload;
pub(crate) mod payload_builder;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct CustomEngineTypes;

impl PayloadTypes for CustomEngineTypes {
    type BuiltPayload = CustomBuiltPayload;
    type PayloadAttributes = EthPayloadAttributes;
    type PayloadBuilderAttributes = EthPayloadBuilderAttributes;
}

impl EngineTypes for CustomEngineTypes {
    type ExecutionPayloadEnvelopeV1 = ExecutionPayloadV1;
    type ExecutionPayloadEnvelopeV2 = ExecutionPayloadEnvelopeV2;
    type ExecutionPayloadEnvelopeV3 = ExecutionPayloadEnvelopeV3;
    type ExecutionPayloadEnvelopeV4 = ExecutionPayloadEnvelopeV4;

    fn block_to_payload(
        block: SealedBlock<
                <<Self::BuiltPayload as reth_node_api::BuiltPayload>::Primitives as reth_node_api::NodePrimitives>::Block,
            >,
    ) -> (ExecutionPayload, ExecutionPayloadSidecar) {
        block_to_payload(block)
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct CustomEngineValidatorBuilder;

impl<N> EngineValidatorBuilder<N> for CustomEngineValidatorBuilder
where
    N: FullNodeComponents<
        Types: NodeTypesWithEngine<
            Engine = CustomEngineTypes,
            ChainSpec = ChainSpec,
            Primitives = CustomPrimitives,
        >,
    >,
{
    type Validator = EthereumEngineValidator;

    async fn build(self, ctx: &AddOnsContext<'_, N>) -> eyre::Result<Self::Validator> {
        Ok(EthereumEngineValidator::new(ctx.config.chain.clone()))
    }
}
