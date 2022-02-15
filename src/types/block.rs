use std::time::SystemTime;

use futures::TryFutureExt;
use serde::{Serialize, Deserialize};
use crate::types::hash::{H256, Hashable};
use rand::Rng;
use super::{merkle::MerkleTree, transaction::SignedTransaction};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header{
    pub parent: H256,
    pub nonce: u32,
    pub difficulty: H256,
    pub timestamp: u128, // 待定
    pub merkle_root: H256, //待定
}
impl Hashable for Header {
    fn hash(&self) -> H256 {
        //unimplemented!() https://docs.rs/ring/0.5.3/ring/digest/fn.digest.html 
        //https://stackoverflow.com/questions/28127165/how-to-convert-struct-to-u8
        let data = bincode::serialize(&self).unwrap(); //u8 type
        return ring::digest::digest(&ring::digest::SHA256, &data).into();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Content{
    pub content: Vec<SignedTransaction>, // 我们之前没有SignedTransaction类型，
                                     //同时需要给SignedTransaction实现hashable
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub content: Content,
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

impl Block {
    pub fn get_parent(&self) -> H256 {
        //unimplemented!()
        self.header.parent
    }

    pub fn get_difficulty(&self) -> H256 {
        //unimplemented!()
        self.header.difficulty
    }
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_block(parent: &H256) -> Block {
    //unimplemented!()
    let mut rng = rand::thread_rng();
    let nonce: u32 = rng.gen();
    let signed_transactions = Vec::new();
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(); //系统时间

    let merkle_tree = MerkleTree::new(&signed_transactions);
    let merkle_root: H256 = merkle_tree.root(); //也可以H256类型

    let mut buffer: [u8; 32] = [0; 32];
    let difficulty: H256 = buffer.into(); //

    let header = Header{ parent: *parent, nonce: nonce, difficulty: difficulty, timestamp: timestamp, merkle_root: merkle_root };
    let content = Content{ content: signed_transactions };
    Block{ header: header, content: content }
}
