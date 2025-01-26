use alloy_consensus::{BlockHeader, Sealable};
use alloy_primitives::{Address, BlockNumber, Bloom, Bytes, B256, B64, U256};
use alloy_rlp::{BufMut, Decodable, Encodable};
use reth::primitives::{serde_bincode_compat::SerdeBincodeCompat, TransactionSigned};
use reth_node_api::NodePrimitives;
use reth_node_core::primitives::InMemorySize;
use serde::{Deserialize, Serialize};

/// Temp helper struct for integrating [`NodePrimitives`].
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct CustomPrimitives;

impl NodePrimitives for CustomPrimitives {
    type Block = reth::primitives::Block<TransactionSigned, CustomHeader>;
    type BlockHeader = CustomHeader;
    type BlockBody = reth::primitives::BlockBody;
    type SignedTx = reth::primitives::TransactionSigned;
    type Receipt = reth::primitives::Receipt;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct CustomHeader {
    eth_header: alloy_consensus::Header,
    extra: Bytes,
}

impl reth_codecs::Compact for CustomHeader {
    fn to_compact<B>(&self, buf: &mut B) -> usize
    where
        B: reth_codecs::__private::bytes::BufMut + AsMut<[u8]>,
    {
        let eth_header_size = self.eth_header.to_compact(buf);
        buf.put_slice(&self.extra);
        eth_header_size + self.extra.len()
    }

    fn from_compact(buf: &[u8], len: usize) -> (Self, &[u8]) {
        let (eth_header, remaining) = alloy_consensus::Header::from_compact(buf, len);

        let extra = Bytes::copy_from_slice(remaining);

        (CustomHeader { eth_header, extra }, &[])
    }
}

impl Encodable for CustomHeader {
    fn encode(&self, out: &mut dyn BufMut) {
        self.eth_header.encode(out);
        out.put_slice(&self.extra);
    }

    fn length(&self) -> usize {
        self.eth_header.length() + self.extra.len()
    }
}

impl Decodable for CustomHeader {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let eth_header = Decodable::decode(buf)?;
        let extra = Decodable::decode(buf)?;
        Ok(Self { eth_header, extra })
    }
}

impl Default for CustomHeader {
    fn default() -> Self {
        Self { eth_header: Default::default(), extra: Default::default() }
    }
}

impl Sealable for CustomHeader {
    fn hash_slow(&self) -> B256 {
        self.eth_header.hash_slow()
    }
}

impl AsRef<Self> for CustomHeader {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl InMemorySize for CustomHeader {
    fn size(&self) -> usize {
        self.eth_header.size() + self.extra.len()
    }
}

impl SerdeBincodeCompat for CustomHeader {
    type BincodeRepr<'a> = alloy_consensus::serde_bincode_compat::Header<'a>;
}

impl<'a> From<&'a CustomHeader> for alloy_consensus::serde_bincode_compat::Header<'a> {
    fn from(value: &'a CustomHeader) -> Self {
        Self::from(&value.eth_header)
    }
}

impl<'a> From<alloy_consensus::serde_bincode_compat::Header<'a>> for CustomHeader {
    fn from(value: alloy_consensus::serde_bincode_compat::Header<'a>) -> Self {
        Self { eth_header: value.into(), extra: Bytes::default() }
    }
}

impl reth_node_core::primitives::BlockHeader for CustomHeader {}

impl BlockHeader for CustomHeader {
    fn parent_hash(&self) -> B256 {
        self.eth_header.parent_hash()
    }

    fn ommers_hash(&self) -> B256 {
        self.eth_header.ommers_hash()
    }

    fn beneficiary(&self) -> Address {
        self.eth_header.beneficiary()
    }

    fn state_root(&self) -> B256 {
        self.eth_header.state_root()
    }

    fn transactions_root(&self) -> B256 {
        self.eth_header.transactions_root()
    }

    fn receipts_root(&self) -> B256 {
        self.eth_header.receipts_root()
    }

    fn withdrawals_root(&self) -> Option<B256> {
        self.eth_header.withdrawals_root()
    }

    fn logs_bloom(&self) -> Bloom {
        self.eth_header.logs_bloom()
    }

    fn difficulty(&self) -> U256 {
        self.eth_header.difficulty()
    }

    fn number(&self) -> BlockNumber {
        self.eth_header.number()
    }

    fn gas_limit(&self) -> u64 {
        self.eth_header.gas_limit()
    }

    fn gas_used(&self) -> u64 {
        self.eth_header.gas_used()
    }

    fn timestamp(&self) -> u64 {
        self.eth_header.timestamp()
    }

    fn mix_hash(&self) -> Option<B256> {
        self.eth_header.mix_hash()
    }

    fn nonce(&self) -> Option<B64> {
        self.eth_header.nonce()
    }

    fn base_fee_per_gas(&self) -> Option<u64> {
        self.eth_header.base_fee_per_gas()
    }

    fn blob_gas_used(&self) -> Option<u64> {
        self.eth_header.blob_gas_used()
    }

    fn excess_blob_gas(&self) -> Option<u64> {
        self.eth_header.excess_blob_gas()
    }

    fn parent_beacon_block_root(&self) -> Option<B256> {
        self.eth_header.parent_beacon_block_root()
    }

    fn requests_hash(&self) -> Option<B256> {
        self.eth_header.requests_hash()
    }

    fn extra_data(&self) -> &Bytes {
        &self.eth_header.extra_data()
    }
}
