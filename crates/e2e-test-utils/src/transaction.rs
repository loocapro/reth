use alloy_consensus::{
    BlobTransactionSidecar, SidecarBuilder, SimpleCoder, TxEip1559, TxEip4844, TxEip4844Variant,
    TxEip4844WithSidecar, TxEnvelope,
};
use alloy_network::{eip2718::Encodable2718, EthereumSigner, TransactionBuilder};
use alloy_rpc_types::TransactionRequest;
use alloy_signer_wallet::LocalWallet;
use eyre::Ok;
use futures_util::{Future, Stream};
use reth_primitives::{
    alloy_primitives::TxKind, constants::eip4844::MAINNET_KZG_TRUSTED_SETUP, hex, Address, Bytes,
    B256, U256,
};
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{sync::Mutex, time::Duration};

/// `TransactionStream` is a struct that represents a stream of transactions.
/// It uses a generator function to create a future for each transaction,
/// and an interval timer to control the rate at which transactions are generated.
pub struct TransactionStream {
    pending_future: Option<Pin<Box<dyn Future<Output = Bytes> + Send + 'static>>>,
    generator: Box<
        dyn Fn() -> Pin<Box<dyn Future<Output = Bytes> + Send + 'static>> + Send + Sync + 'static,
    >,
    interval: tokio::time::Interval,
}

impl TransactionStream {
    /// Creates a new `TransactionStream` with the given generator function.
    /// The generator function is used to create a future for each transaction.
    ///
    /// # Arguments
    ///
    /// * `generator` - A function that returns a `Future` that resolves to a `Bytes`.
    pub fn new<F, Fut>(generator: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Bytes> + Send + 'static,
    {
        // 100 tx per second as default
        let interval = Duration::from_secs_f64(1.0 / 100.0);
        TransactionStream {
            generator: Box::new(move || Box::pin(generator())),
            pending_future: None,
            interval: tokio::time::interval(interval),
        }
    }

    /// Sets the rate at which transactions are generated.
    ///
    /// # Arguments
    ///
    /// * `transactions_per_second` - The number of transactions to generate per second.
    pub fn tx_per_sec(mut self, transactions_per_second: f64) -> Self {
        let interval = Duration::from_secs_f64(1.0 / transactions_per_second);
        self.interval = tokio::time::interval(interval);
        self
    }

    /// Generates a new future if there is no pending future.
    fn generate_future(&mut self) {
        if self.pending_future.is_none() {
            self.pending_future = Some((self.generator)());
        }
    }
}

impl Stream for TransactionStream {
    type Item = Bytes;

    /// Polls the next item in the stream.
    ///
    /// If there is a pending future, it will be polled. If the future is ready,
    /// it will be taken and the result will be returned. If the future is not ready,
    /// `Poll::Pending` will be returned.
    ///
    /// If there is no pending future, a new one will be generated using the generator function.
    ///
    /// The method will continue to loop until the interval timer ticks, at which point
    /// it will return `Poll::Pending` and wait for the next tick to generate a new transaction.
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let Some(pending_future) = self.pending_future.as_mut() {
                if let Poll::Ready(res) = pending_future.as_mut().poll(cx) {
                    self.pending_future = None;
                    return Poll::Ready(Some(res));
                }
            }

            self.generate_future();

            if self.interval.poll_tick(cx).is_ready() {
                continue;
            } else {
                return Poll::Pending;
            }
        }
    }
}

/// `TransactionTestContext` is a structure that provides a context for testing transactions.
/// It contains a `chain_id` to identify the blockchain network, a `wallet` for signing
/// transactions, and a `nonce` for ensuring transactions are processed in order.
#[derive(Clone)]
pub struct TransactionTestContext {
    chain_id: u64,
    wallet: LocalWallet,
    pub nonce: Arc<Mutex<u64>>,
}

impl TransactionTestContext {
    /// Creates a new `TransactionTestContext` with the given `chain_id` and `wallet`.
    pub fn new(chain_id: u64, wallet: LocalWallet) -> Self {
        Self { chain_id, wallet, nonce: Arc::new(Mutex::new(0)) }
    }

    /// Increments the `nonce` by one.
    pub async fn inc_nonce(&self) {
        let mut guard = self.nonce.lock().await;
        *guard += 1;
    }

    /// Creates a new EIP-1559 transaction and returns its encoded form.
    pub async fn eip1559(&mut self) -> Bytes {
        let to = Address::random();
        let nonce = *self.nonce.lock().await;
        let tx = TxEip1559 {
            chain_id: self.chain_id,
            nonce,
            gas_limit: 21_000,
            to: TxKind::Call(to),
            max_priority_fee_per_gas: 25e9 as u128,
            max_fee_per_gas: 25e9 as u128,
            value: U256::from(1),
            ..Default::default()
        };

        let mut tx_request: TransactionRequest = tx.into();
        tx_request.access_list = None;

        self.sign_and_encode(tx_request).await.unwrap()
    }

    /// Creates a new EIP-4844 transaction and returns its encoded form.
    pub async fn eip4844(&mut self) -> eyre::Result<Bytes> {
        let tx = self.dummy_eip4844().await;
        let tx_request: TransactionRequest = tx.into();

        self.sign_and_encode(tx_request).await
    }

    /// Creates a new Optimism block info transaction and returns its encoded form.
    pub async fn optimism_block_info(&mut self) -> Bytes {
        let data = Bytes::from_static(&hex!("7ef9015aa044bae9d41b8380d781187b426c6fe43df5fb2fb57bd4466ef6a701e1f01e015694deaddeaddeaddeaddeaddeaddeaddeaddead000194420000000000000000000000000000000000001580808408f0d18001b90104015d8eb900000000000000000000000000000000000000000000000000000000008057650000000000000000000000000000000000000000000000000000000063d96d10000000000000000000000000000000000000000000000000000000000009f35273d89754a1e0387b89520d989d3be9c37c1f32495a88faf1ea05c61121ab0d1900000000000000000000000000000000000000000000000000000000000000010000000000000000000000002d679b567db6187c0c8323fa982cfb88b74dbcc7000000000000000000000000000000000000000000000000000000000000083400000000000000000000000000000000000000000000000000000000000f4240"));
        let nonce = *self.nonce.lock().await;
        let tx = TxEip1559 {
            chain_id: self.chain_id,
            nonce,
            gas_limit: 210_000,
            to: TxKind::Call(Address::random()),
            max_priority_fee_per_gas: 20e9 as u128,
            max_fee_per_gas: 20e9 as u128,
            value: U256::from(1),
            input: data,
            ..Default::default()
        };
        let mut tx_req: TransactionRequest = tx.into();
        tx_req.access_list = None;
        self.sign_and_encode(tx_req).await.unwrap()
    }

    /// Signs and encodes a transaction request.
    async fn sign_and_encode(&mut self, tx: TransactionRequest) -> eyre::Result<Bytes> {
        let signer = EthereumSigner::from(self.wallet.clone());
        let signed = tx.build(&signer).await?;
        self.inc_nonce().await;
        Ok(signed.encoded_2718().into())
    }

    /// Creates a dummy EIP-4844 transaction for testing purposes.
    async fn dummy_eip4844(&self) -> TxEip4844WithSidecar {
        let nonce = *self.nonce.lock().await;
        let tx = TxEip4844 {
            chain_id: self.chain_id,
            nonce,
            max_priority_fee_per_gas: 20e9 as u128,
            max_fee_per_gas: 20e9 as u128,
            gas_limit: 21_000,
            to: Default::default(),
            value: U256::from(1),
            access_list: Default::default(),
            blob_versioned_hashes: vec![Default::default()],
            max_fee_per_blob_gas: 1,
            input: Default::default(),
        };

        let mut builder = SidecarBuilder::<SimpleCoder>::new();
        builder.ingest(b"dummy blob");
        let sidecar: BlobTransactionSidecar = builder.build().unwrap();

        TxEip4844WithSidecar { tx, sidecar }
    }

    /// Validates the sidecar of a given transaction envelope and returns the versioned hashes.
    pub fn validate_sidecar(tx: TxEnvelope) -> Vec<B256> {
        let proof_setting = MAINNET_KZG_TRUSTED_SETUP.clone();

        match tx {
            TxEnvelope::Eip4844(signed) => match signed.tx() {
                TxEip4844Variant::TxEip4844WithSidecar(tx) => {
                    tx.validate_blob(&proof_setting).unwrap();
                    tx.sidecar.versioned_hashes().collect()
                }
                _ => panic!("Expected Eip4844 transaction with sidecar"),
            },
            _ => panic!("Expected Eip4844 transaction"),
        }
    }
}
