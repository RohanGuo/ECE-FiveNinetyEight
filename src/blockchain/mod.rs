use crate::types::block::{Block, Header, Content};
use crate::types::hash::{H256, Hashable};
use std::collections::HashMap;
use crate::types::merkle::MerkleTree;
use rand::Rng;
use hex_literal::hex;
use std::sync::{Arc, Mutex};
use crate::types::transaction::*;

pub struct Blockchain {
    pub block_map: HashMap<H256, Block>,
    pub block_seq: HashMap<H256, usize>,
    pub tip: H256,
    pub state: Arc<Mutex<State>>,

}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
        pub fn new(state: &Arc<Mutex<State>>,) -> Self {
            let mut rng = rand::thread_rng();
            let parent = [0; 32].into();
            let nonce = 0u32;
            let signed_transactions = Vec::new();
            let timestamp = 0u128; //系统时间

            let merkle_tree = MerkleTree::new(&signed_transactions);
            let merkle_root: H256 = merkle_tree.root(); //也可以H256类型

            let difficulty: H256 = hex!("0005511111111111111111111111111111111111111111111111111111111111").into();
            let header = Header{ parent: parent, nonce: nonce, difficulty: difficulty, timestamp: timestamp, merkle_root: merkle_root };
            let content = Content{ content: signed_transactions};
            let genesis = Block{ header: header, content: content};
            let genesis_hash = genesis.hash();
            let mut block_map = HashMap::new();
            let mut block_seq = HashMap::new();
            let mut state_locked = state.lock().unwrap();
            state_locked.update(&genesis.clone());
            block_map.insert(genesis_hash, genesis);
            block_seq.insert(genesis_hash, 0);
            
            Blockchain {block_map: block_map, block_seq: block_seq, tip: genesis_hash, state: state.clone(),}
        }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let parent = block.get_parent();
        let block_hash = block.hash();
        self.block_map.insert(block_hash, block.clone());
        self.block_seq.insert(block_hash, self.block_seq[&parent] + 1);
        if self.block_seq[&block_hash] > self.block_seq[&self.tip] {
            self.tip = block_hash;
        }
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        self.tip
    }

    /// Get all blocks' hashes of the longest chain, ordered from genesis to the tip
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut chain = Vec::new();
        let mut hash = self.tip;
        while self.block_seq[&hash] != 0 {
            chain.push(hash);
            hash = self.block_map[&hash].get_parent();
        }
        chain.push(hash);
        chain.reverse();
        return chain;
    }

    pub fn all_transactions_in_longest_chain(&self) -> Vec<Vec<String>> {
        let mut chain = Vec::new();
        let mut hash = self.tip;
        while self.block_seq[&hash] != 0 {
            chain.push(self.block_map[&hash].get_transactions());
            hash = self.block_map[&hash].get_parent();
        }
        chain.push(self.block_map[&hash].get_transactions());
        chain.reverse();
        return chain;
    }
    // through account history
    pub fn get_accounts_by_block_number(&self, block_hash: H256) -> Vec<String> {
        let state = self.state.lock().unwrap();
        let mut accounts: Vec<String> = Vec::new();
        let accounts_his = &state.history;
        let a = accounts_his.get(&block_hash);
        let b = match a {
            Some(v) => v,
            None => {
                println!("error");
                return Vec::new();
            }
        };
        for (account, info) in b {
            accounts.push("Account: ".to_owned() + &account.to_string() + "; Nonce: " + &info.0.to_string() + "; Balance: " + &info.1.to_string());
        }
        return accounts;
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::types::block::generate_random_block;
//     use crate::types::hash::Hashable;

//     #[test]
//     fn insert_one() {
//         let mut blockchain = Blockchain::new();
//         let genesis_hash = blockchain.tip();
//         let block = generate_random_block(&genesis_hash);
//         blockchain.insert(&block);
//         assert_eq!(blockchain.tip(), block.hash());

//     }
// }

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST