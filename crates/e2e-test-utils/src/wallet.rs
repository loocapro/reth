use crate::transaction::TransactionTestContext;
use alloy_signer::Signer;
use alloy_signer_wallet::{coins_bip39::English, LocalWallet, MnemonicBuilder};
use reth_primitives::{Bytes, ChainId, MAINNET};

/// Default test mnemonic used by the accounts in the test genesis allocations
const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";

/// Wallet generator that can generate wallets from a given phrase, chain id and amount
#[derive(Clone, Debug)]
pub struct WalletGenerator {
    chain_id: u64,
    phrase: String,
    derivation_path: String,
}

impl Default for WalletGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl WalletGenerator {
    /// Creates a new wallet generator defaulting to MAINNET specs
    pub fn new() -> Self {
        Self {
            chain_id: 1,
            phrase: TEST_MNEMONIC.to_string(),
            derivation_path: "m/44'/60'/0'/0/".to_string(),
        }
    }

    /// Sets the mnemonic phrase that will be used to generate the wallets
    pub fn phrase(mut self, phrase: impl Into<String>) -> Self {
        self.phrase = phrase.into();
        self
    }

    /// Sets the chain id that will be used to generate the wallets
    pub fn chain_id(mut self, chain_id: impl Into<u64>) -> Self {
        self.chain_id = chain_id.into();
        self
    }

    /// Sets the derivation path that will be used to generate the wallets, following the BIP44
    /// <https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki>
    pub fn derivation_path(mut self, derivation_path: impl Into<String>) -> Self {
        let mut path = derivation_path.into();
        if !path.ends_with('/') {
            path.push('/');
        }
        self.derivation_path = path;
        self
    }

    fn get_derivation_path(&self, index: usize) -> String {
        format!("m/44'/60'/0'/0/{}", index)
    }
}

impl WalletGenerator {
    /// Generates a single wallet from a previously set phrase and chain id
    pub fn gen(&self) -> Wallet {
        self.generate_wallet(0)
    }

    /// Generates multiple wallets from a previously set phrase, chain id and amount
    pub fn gen_many(&self, amount: usize) -> Vec<Wallet> {
        (0..amount).map(|idx| self.generate_wallet(idx)).collect()
    }

    /// Helper function to generate a wallet for a given index
    fn generate_wallet(&self, index: usize) -> Wallet {
        let builder = MnemonicBuilder::<English>::default().phrase(self.phrase.as_str());

        // use the derivation path
        let derivation_path = self.get_derivation_path(index);

        let builder = builder.derivation_path(derivation_path).unwrap();
        let inner = builder.build().unwrap().with_chain_id(Some(self.chain_id));
        Wallet::new(inner, self.chain_id)
    }
}
/// Helper struct that wraps interaction to a local wallet and a transaction generator
#[derive(Clone)]
pub struct Wallet {
    inner: LocalWallet,
    pub tx_gen: TransactionTestContext,
}

impl Wallet {
    /// Create a new wallet with a given chain id
    pub fn new(inner: LocalWallet, chain_id: u64) -> Self {
        let tx_gen = TransactionTestContext::new(chain_id, inner.clone());
        Self { inner, tx_gen }
    }
    /// Get the address of the wallet
    pub fn address(&self) -> String {
        self.inner.address().to_string()
    }

    /// Get an EIP1559 transaction
    pub async fn eip1559(&mut self) -> Bytes {
        self.tx_gen.eip1559().await
    }

    /// Get an EIP4844 transaction
    pub async fn eip4844(&mut self) -> Bytes {
        self.tx_gen.eip4844().await.unwrap()
    }
    /// Get an optimism block info transaction
    pub async fn optimism_block_info(&mut self) -> Bytes {
        self.tx_gen.optimism_block_info().await
    }
}

/// As deafult we use the test mnemonic and mainnet specs.
impl Default for Wallet {
    fn default() -> Self {
        let builder = MnemonicBuilder::<English>::default().phrase(TEST_MNEMONIC);
        let inner = builder.build().unwrap();
        let chain_id: ChainId = MAINNET.chain.into();
        let tx_gen = TransactionTestContext::new(chain_id, inner.clone());
        Self { inner, tx_gen }
    }
}
