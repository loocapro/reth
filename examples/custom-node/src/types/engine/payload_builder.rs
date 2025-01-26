use crate::types::primitives::CustomPrimitives;

use super::{built_payload::CustomBuiltPayload, CustomEngineTypes};
use reth::{
    builder::{components::PayloadServiceBuilder, BuilderContext, PayloadBuilderConfig},
    primitives::TransactionSigned,
    providers::StateProviderFactory,
    transaction_pool::{PoolTransaction, TransactionPool},
};
use reth_basic_payload_builder::{
    BasicPayloadJobGenerator, BasicPayloadJobGeneratorConfig, BuildArguments, BuildOutcome,
    PayloadBuilder, PayloadConfig,
};
use reth_chain_state::CanonStateSubscriptions;
use reth_chainspec::{ChainSpec, ChainSpecProvider};
use reth_ethereum_payload_builder::EthereumBuilderConfig;
use reth_node_api::{FullNodeTypes, NodeTypesWithEngine, PayloadBuilderError};
use reth_node_core::version::default_extra_data_bytes;
use reth_node_ethereum::EthEvmConfig;
use reth_payload_builder::{
    EthPayloadBuilderAttributes, PayloadBuilderHandle, PayloadBuilderService,
};

/// A custom payload service builder that supports the custom engine types
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub(crate) struct CustomPayloadServiceBuilder;

impl<Node, Pool> PayloadServiceBuilder<Node, Pool> for CustomPayloadServiceBuilder
where
    Node: FullNodeTypes<
        Types: NodeTypesWithEngine<
            Engine = CustomEngineTypes,
            ChainSpec = ChainSpec,
            Primitives = CustomPrimitives,
        >,
    >,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TransactionSigned>>
        + Unpin
        + 'static,
{
    async fn spawn_payload_service(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<PayloadBuilderHandle<<Node::Types as NodeTypesWithEngine>::Engine>> {
        let payload_builder = CustomPayloadBuilder::default();
        let conf = ctx.payload_builder_config();

        let payload_job_config = BasicPayloadJobGeneratorConfig::default()
            .interval(conf.interval())
            .deadline(conf.deadline())
            .max_payload_tasks(conf.max_payload_tasks());

        let payload_generator = BasicPayloadJobGenerator::with_builder(
            ctx.provider().clone(),
            pool,
            ctx.task_executor().clone(),
            payload_job_config,
            payload_builder,
        );
        let (payload_service, payload_builder) =
            PayloadBuilderService::new(payload_generator, ctx.provider().canonical_state_stream());

        ctx.task_executor().spawn_critical("payload builder service", Box::pin(payload_service));

        Ok(payload_builder)
    }
}

/// The type responsible for building custom payloads
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct CustomPayloadBuilder;

impl<Pool, Client> PayloadBuilder<Pool, Client> for CustomPayloadBuilder
where
    Client: StateProviderFactory + ChainSpecProvider<ChainSpec = ChainSpec>,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TransactionSigned>>,
{
    type Attributes = EthPayloadBuilderAttributes;
    type BuiltPayload = CustomBuiltPayload;

    fn try_build(
        &self,
        args: BuildArguments<Pool, Client, Self::Attributes, Self::BuiltPayload>,
    ) -> Result<BuildOutcome<Self::BuiltPayload>, PayloadBuilderError> {
        let BuildArguments { cached_reads, best_payload, .. } = args;

        let payload = best_payload.unwrap();

        Ok(BuildOutcome::Better { payload, cached_reads })
    }

    fn build_empty_payload(
        &self,
        client: &Client,
        config: PayloadConfig<Self::Attributes>,
    ) -> Result<Self::BuiltPayload, PayloadBuilderError> {
        let PayloadConfig { parent_header, attributes } = config;

        let chain_spec = client.chain_spec();
        let empty_payload =
            <reth_ethereum_payload_builder::EthereumPayloadBuilder as PayloadBuilder<
                Pool,
                Client,
            >>::build_empty_payload(
                &reth_ethereum_payload_builder::EthereumPayloadBuilder::new(
                    EthEvmConfig::new(chain_spec.clone()),
                    EthereumBuilderConfig::new(default_extra_data_bytes()),
                ),
                client,
                PayloadConfig { parent_header, attributes },
            )?;

        Ok(CustomBuiltPayload::new(empty_payload))
    }
}
