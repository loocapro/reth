use engine::CustomEngineTypes;
use primitives::CustomPrimitives;
use reth::{builder::node::NodeTypes, providers::EthStorage};
use reth_chainspec::ChainSpec;
use reth_node_api::NodeTypesWithEngine;
use reth_trie_db::MerklePatriciaTrie;

use crate::node::MyCustomNode;

pub(crate) mod engine;
pub(crate) mod primitives;

/// Configure the node types with the custom engine types
impl NodeTypesWithEngine for MyCustomNode {
    type Engine = CustomEngineTypes;
}

/// Configure the node types
impl NodeTypes for MyCustomNode {
    type Primitives = CustomPrimitives;
    type ChainSpec = ChainSpec;
    type StateCommitment = MerklePatriciaTrie;
    type Storage = EthStorage;
}
