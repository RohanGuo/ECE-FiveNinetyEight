use serde::{Serialize,Deserialize};
use crate::types::key_pair;
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use rand::{Rng, prelude::SliceRandom, thread_rng};
use ring::digest;
//use crate::types::{H256, Hashable};
use super::{address::{Address, self}, hash::{Hashable, H256}};
use std::{collections::{HashMap, HashSet}, ops::Add};
use crate::types::block::*;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    nonce: u32, // the nonce after transaction
    sender: Address,
    receiver: Address,
    value: i32,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let data = bincode::serialize(&self).unwrap();
        digest::digest(&digest::SHA256, &data).into()
    }
}

pub fn generate_random_signed_transaction(sender: Address, receiver: Address, value: i32, nonce: u32, key: &Ed25519KeyPair) -> SignedTransaction {
    let transaction = generate_random_transaction(sender, receiver, value, nonce);
    // let key = key_pair::random();
    let signature = sign(&transaction, &key);
    SignedTransaction {
        transaction: transaction, 
        public_key: key.public_key().as_ref().to_vec(), 
        signature: signature.as_ref().to_vec()
    }
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    let bytes = bincode::serialize(t).unwrap();
    key.sign(&bytes)
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(tx: &Transaction, public_key: &[u8], signature: &[u8], state: &State) -> bool {
    // let public_key =
    //     ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519,
    //                                   &public_key);
    // let transaction_key = bincode::serialize(&tx).unwrap();            
    // let check1 = public_key.verify(&transaction_key, signature.as_ref()).is_ok();

    // let (ac_nonce,balance) = state.accounts[&tx.sender]; 
    // let check2 = (balance-tx.value)>0;
    // let check3 = ac_nonce == tx.nonce + 1;
    // return check1 && check2 && check3
    
    // todo: implement verify function
    return true
    
}

pub struct TransactionMemopool {
    pub trans_map: HashMap<H256, SignedTransaction>,
}

impl TransactionMemopool {
    pub fn new() -> Self {
        let mut trans_map: HashMap<H256, SignedTransaction> = HashMap::new();
        TransactionMemopool {
            trans_map: trans_map, 
        }
    }
}
pub struct State {
    pub accounts: HashMap<Address, (u32, i32)>, //address, (nonce, balance)
    pub history: HashMap<H256, HashMap<Address, (u32, i32)>>,
}

pub fn generate_address() -> Address {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        const LENGTH: i32 = 64;
        let mut rng = rand::thread_rng();
        let address_pub_key: String = (0..LENGTH)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
    
        let address_pub_key = address_pub_key.as_bytes();
        let addr: Address =  Address::from_public_key_bytes(&address_pub_key);
        addr
}

impl State {
    pub fn new(addr: Address) -> Self {
        let mut accounts: HashMap<Address, (u32,i32)> = HashMap::new();
        let mut history: HashMap<H256, HashMap<Address, (u32, i32)>> = HashMap::new();
        let nonce = 0;
        let balance = 1000;
        accounts.insert(addr, (nonce, balance));
        State{
            accounts: accounts,
            history: history,
        }
    }
    // change 
    pub fn update(&mut self, block: &Block) -> bool {
        let mut accounts = self.accounts.clone();
        let mut isValid = true;
        for signedTrans in &block.content.content {
            // todo: add check here: trans verify
            let check1 = false;
            let trans = signedTrans.transaction.clone();
            let receiver = trans.receiver;
            let sender = trans.sender;
            let nonce = trans.nonce;
            let value = trans.value;
            if receiver == sender {
                if !accounts.contains_key(&receiver) {
                    accounts.insert(receiver, (nonce, value));
                    // println!("account {:?} value {:?} nonce {:?}", receiver, value, nonce);
                }
            } else {
                // todo: add other cases
                let mut old_receiver_account = accounts.get(&receiver);
                let mut receiver_account = match old_receiver_account  {
                    Some(v) => v,
                    None => {
                        continue;
                    }
                };
                let mut old_sender_account = accounts.get(&sender);
                let mut sender_account = match old_sender_account  {
                    Some(v) => v,
                    None => {
                        continue;
                    }
                };
                
                if sender_account.0 + 1 != nonce {
                    continue;
                }
                if sender_account.1 < value {
                    continue;
                }
                let mut sender_account = *sender_account;
                sender_account.0 = sender_account.0 + 1;
                sender_account.1 -= value;
                let mut receiver_account = *receiver_account;
                receiver_account.1 += value;
                // println!("sender {:?} nonce {:?} value {:?} receiver {:?} value {:?} tx_value {:?}", sender, sender_account.0, sender_account.1, receiver, receiver_account.1, value);
                accounts.insert(sender, sender_account.clone());
                accounts.insert(receiver, receiver_account.clone());
            }
        }
        if isValid {
            self.history.insert(block.hash(), accounts.clone());
            self.accounts = accounts;
        }
        return isValid;



        // // let tx = signedT.transaction.clone();
        // let mut res = true;
        // let mut ac_nonce = self.accounts[&tx.sender].0;
        // let balance = self.accounts[&tx.sender].1;
        // let new_balance = balance - tx.value;    
        // if new_balance >=0 {
        //     ac_nonce -= 1;
        //     self.accounts.insert(tx.sender, (ac_nonce,new_balance));
        //     // update reverver balance
        //     let (r_nonce, r_balance) = self.accounts[&tx.receiver];
        //     let r_new_balance = r_balance + tx.value;
        //     self.accounts.insert(tx.receiver, (r_nonce, r_new_balance));
        // }else{
        //     res = false;
        // }
        // return res;
    }
}

// #[cfg(any(test, test_utilities))]
pub fn generate_random_transaction(sender: Address, receiver: Address, value: i32, nonce: u32) -> Transaction {
    return Transaction{nonce, sender, receiver, value};
    
    
    // let mut rng = rand::thread_rng();
    // let n1: u8 = rng.gen();
    // let addr1:Address = [1u8; 20].into();
    // let addr2:Address = [1u8; 20].into();
    // let addr3:Address = [1u8; 20].into();
    // let mut addr_vec = vec![addr1,addr2,addr3];
    // let sender = addr_vec.remove(0);
    // let receiver = addr_vec.remove(0);
    // let value:i32 = (n1%10).into();
    // let nonce = 10;
    // return Transaction{nonce, sender, receiver, value}
    // const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    // const LENGTH: i32 = 64;
    // let mut rng = rand::thread_rng();
    // let address1: String = (0..LENGTH)
    //     .map(|_| {
    //         let idx = rng.gen_range(0..CHARSET.len());
    //         CHARSET[idx] as char
    //     })
    //     .collect();
    // let address2: String = (0..LENGTH)
    //     .map(|_| {
    //         let idx = rng.gen_range(0..CHARSET.len());
    //         CHARSET[idx] as char
    //     })
    //     .collect();
    // // println!("{}, {}", address1, address2);
    // let address1 = address1.as_bytes();
    // let address2 = address2.as_bytes();
    // let sender: Address = Address::from_public_key_bytes(&address1);
    // let receiver: Address = Address::from_public_key_bytes(&address2);
    // // let value = rand::thread_rng().gen::<i32>();
    // Transaction{nonce, sender, receiver, value}
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::types::key_pair;
//     use ring::signature::KeyPair;


//     #[test]
//     fn sign_verify() {
//         let t = generate_random_transaction();
//         let key = key_pair::random();
//         let signature = sign(&t, &key);
//         let result = verify(&t, key.public_key().as_ref(), signature.as_ref());
//         // println!("{:?}", result);
//         assert!(verify(&t, key.public_key().as_ref(), signature.as_ref()));
//     }
//     #[test]
//     fn sign_verify_two() {
//         let t = generate_random_transaction();
//         let key = key_pair::random();
//         let signature = sign(&t, &key);
//         let key_2 = key_pair::random();
//         let t_2 = generate_random_transaction();
//         assert!(!verify(&t_2, key.public_key().as_ref(), signature.as_ref()));
//         assert!(!verify(&t, key_2.public_key().as_ref(), signature.as_ref()));
//     }
// }

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
