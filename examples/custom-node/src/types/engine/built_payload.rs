use alloy_eips::eip7685::Requests;
use alloy_primitives::U256;
use alloy_rpc_types::engine::{
    ExecutionPayloadEnvelopeV2, ExecutionPayloadEnvelopeV3, ExecutionPayloadEnvelopeV4,
    ExecutionPayloadV1,
};
use reth::{
    primitives::SealedBlock,
    rpc::compat::engine::{
        block_to_payload_v1,
        payload::{block_to_payload_v3, convert_block_to_payload_field_v2},
    },
};
use reth_chain_state::ExecutedBlock;
use reth_node_api::BuiltPayload;
use reth_payload_builder::EthBuiltPayload;

use crate::types::primitives::CustomPrimitives;

#[derive(Debug, Clone)]
pub struct CustomBuiltPayload(pub(crate) EthBuiltPayload);

impl AsRef<EthBuiltPayload> for CustomBuiltPayload {
    fn as_ref(&self) -> &EthBuiltPayload {
        &self.0
    }
}

impl BuiltPayload for CustomBuiltPayload {
    type Primitives = CustomPrimitives;

    fn block(&self) -> &SealedBlock {
        &self.as_ref().block()
    }

    fn fees(&self) -> U256 {
        self.as_ref().fees()
    }

    fn executed_block(&self) -> Option<ExecutedBlock<CustomPrimitives>> {
        //TODO: Implement this
        None
    }

    fn requests(&self) -> Option<Requests> {
        self.as_ref().requests().clone()
    }
}

// V1 engine_getPayloadV1 response
impl From<CustomBuiltPayload> for ExecutionPayloadV1 {
    fn from(value: CustomBuiltPayload) -> Self {
        let block = value.as_ref().block().clone();
        block_to_payload_v1(block)
    }
}

// V2 engine_getPayloadV2 response
impl From<CustomBuiltPayload> for ExecutionPayloadEnvelopeV2 {
    fn from(value: CustomBuiltPayload) -> Self {
        let built_payload = value.as_ref();

        Self {
            block_value: built_payload.fees(),
            execution_payload: convert_block_to_payload_field_v2(built_payload.block().clone()),
        }
    }
}

impl From<CustomBuiltPayload> for ExecutionPayloadEnvelopeV3 {
    fn from(value: CustomBuiltPayload) -> Self {
        let built_payload = value.as_ref();
        let block = built_payload.block().clone();
        let fees = built_payload.fees();
        let sidecars = built_payload.sidecars().to_vec();

        Self {
            execution_payload: block_to_payload_v3(block),
            block_value: fees,
            // From the engine API spec:
            //
            // > Client software **MAY** use any heuristics to decide whether to set
            // `shouldOverrideBuilder` flag or not. If client software does not implement any
            // heuristic this flag **SHOULD** be set to `false`.
            //
            // Spec:
            // <https://github.com/ethereum/execution-apis/blob/fe8e13c288c592ec154ce25c534e26cb7ce0530d/src/engine/cancun.md#specification-2>
            should_override_builder: false,
            blobs_bundle: sidecars.into(),
        }
    }
}

impl From<CustomBuiltPayload> for ExecutionPayloadEnvelopeV4 {
    fn from(value: CustomBuiltPayload) -> Self {
        Self {
            execution_requests: value.as_ref().requests().clone().unwrap_or_default(),
            envelope_inner: value.into(),
        }
    }
}
