use serde::{Serialize,Deserialize, serde_if_integer128};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use rand::Rng;

use super::address::Address;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    pub sender: Address, //[u8,20]
    pub receiver: Address,
    pub value: i32,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    //reference: https://docs.rs/ring/latest/ring/signature/index.html
    let ttt = bincode::serialize(t).unwrap();
    let signat = key.sign(&ttt);
    // t.signature = true;
    signat
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &[u8], signature: &[u8]) -> bool {
    let peer_public_key_bytes = public_key;
    let peer_public_key = ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, peer_public_key_bytes);
    
    let t_s = bincode::serialize(&t).unwrap();
    peer_public_key.verify(&t_s, signature.as_ref()).is_ok()
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_transaction() -> Transaction {
    //https://rust-cookbook.budshome.com/algorithms/randomness.html
    let mut buffer1: [u8; 20] = [0; 20];  //初始化，全零
    for idx in 0..20 {
        let mut rng = rand::thread_rng();
        let n1: u8 = rng.gen();
        //copy_from_slice()
        buffer1[idx] = n1;
    }
    let sender = buffer1.into(); // Address(buffer);
    let mut buffer2: [u8; 20] = [0; 20];
    for idx in 0..20 {
        let mut rng = rand::thread_rng();
        let n1: u8 = rng.gen();
        //copy_from_slice()
        buffer2[idx] = n1;
    }
    let receiver = buffer2.into();
    let value = rand::thread_rng().gen::<i32>();
    Transaction{sender,receiver,value}
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::key_pair;
    use ring::signature::KeyPair;


    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, key.public_key().as_ref(), signature.as_ref()));
    }
    #[test]
    fn sign_verify_two() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        let key_2 = key_pair::random();
        let t_2 = generate_random_transaction();
        assert!(!verify(&t_2, key.public_key().as_ref(), signature.as_ref()));
        assert!(!verify(&t, key_2.public_key().as_ref(), signature.as_ref()));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
