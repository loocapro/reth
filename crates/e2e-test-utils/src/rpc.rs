use crate::transaction::TransactionStream;
use alloy_consensus::TxEnvelope;
use alloy_network::eip2718::Decodable2718;
use futures_util::StreamExt;
use reth::{api::FullNodeComponents, builder::rpc::RpcRegistry, rpc::api::DebugApiServer};
use reth_primitives::{Bytes, B256};
use reth_rpc::eth::{error::EthResult, EthTransactions};
use tracing::error;

pub struct RpcTestContext<Node: FullNodeComponents> {
    pub inner: RpcRegistry<Node>,
}

impl<Node: FullNodeComponents> RpcTestContext<Node> {
    /// Injects a raw transaction into the node tx pool via RPC server
    pub async fn inject_tx(&mut self, raw_tx: Bytes) -> EthResult<B256> {
        let eth_api = self.inner.eth_api();
        eth_api.send_raw_transaction(raw_tx).await
    }

    /// Asynchronously injects a stream of transactions into the node pool.
    ///
    /// This function takes a stream of raw transactions (in `Bytes` format) and sends each
    /// transaction to the Ethereum API for processing. Each transaction is sent in its own
    /// asynchronous task, allowing for concurrent processing of multiple transactions.
    ///
    /// Note: This function does not block and returns immediately after spawning the necessary
    /// tasks.
    pub fn inject_stream(&mut self, stream: TransactionStream) {
        let eth_api = self.inner.eth_api();
        tokio::spawn(stream.for_each(move |raw_tx| {
            let eth_api_clone = eth_api.clone();
            async move {
                if let Err(e) = eth_api_clone.send_raw_transaction(raw_tx).await {
                    error!(?e, "Error injecting tx");
                }
            }
        }));
    }

    /// Retrieves a transaction envelope by its hash
    pub async fn envelope_by_hash(&mut self, hash: B256) -> eyre::Result<TxEnvelope> {
        let tx = self.inner.debug_api().raw_transaction(hash).await?.unwrap();
        let tx = tx.to_vec();
        Ok(TxEnvelope::decode_2718(&mut tx.as_ref()).unwrap())
    }
}
