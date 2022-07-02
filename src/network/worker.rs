use super::message::Message;
use super::peer;
use super::server::Handle as ServerHandle;
use crate::blockchain::Blockchain;
use crate::types::block::Block;
use crate::types::hash::{H256, Hashable};
use crate::types::transaction::{TransactionMemopool, State};
use crate::types::transaction::SignedTransaction;
use crate::types::transaction::verify;
use log::{debug, warn, error};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(any(test,test_utilities))]
use super::peer::TestReceiver as PeerTestReceiver;
#[cfg(any(test,test_utilities))]
use super::server::TestReceiver as ServerTestReceiver;
#[derive(Clone)]
pub struct Worker {
    msg_chan: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    orph_buff: Arc<Mutex<HashMap<H256, Block>>>,
    trans_memopool: Arc<Mutex<TransactionMemopool>>,
    state: Arc<Mutex<State>>,
}


impl Worker {
    pub fn new(
        num_worker: usize,
        msg_src: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
        server: &ServerHandle,
        blockchain: &Arc<Mutex<Blockchain>>,
        orph_buff: &Arc<Mutex<HashMap<H256, Block>>>,
        trans_memopool: &Arc<Mutex<TransactionMemopool>>,
        state: &Arc<Mutex<State>>,
    ) -> Self {
        Self {
            msg_chan: msg_src,
            num_worker,
            server: server.clone(),
            blockchain: Arc::clone(blockchain),
            orph_buff: Arc::clone(orph_buff),
            trans_memopool: Arc::clone(trans_memopool),
            state: Arc::clone(state)
        }
    }

    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {
        loop {
            let result = smol::block_on(self.msg_chan.recv());
            if let Err(e) = result {
                error!("network worker terminated {}", e);
                break;
            }
            let msg = result.unwrap();
            let (msg, mut peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                // receive msg type, peer is for write
                //_ => unimplemented!(),

                // receive hashes, find hashes not in blockchain
                Message::NewBlockHashes(nonce) =>{
                    if nonce.len() != 0{
                        let mut vec_hash: Vec<H256> = Vec::new();
                        let blockchain = self.blockchain.lock().unwrap();
                        for hash in nonce.clone() {   
                            if blockchain.block_map.get(&hash).is_none(){
                                vec_hash.push(hash);
                            }
                        }
                        if vec_hash.len() != 0 {
                            peer.write(Message::GetBlocks(vec_hash));
                        }
                    }
                }
                //receive hashes and reply Blocks.
                Message::GetBlocks(nonce) => {
                    let blockchain = self.blockchain.lock().unwrap();
                    let mut blocks = nonce.clone();
                    let mut vec = Vec::new();
                    for i in 0..blocks.len() {
                        let hash = blocks[i];
                        if !blockchain.block_map.get(&hash).is_none() {
                            vec.push(blockchain.block_map.get(&hash).unwrap().clone());
                        }
                    }
                    if vec.len() != 0{
                        peer.write(Message::Blocks(vec));
                    }
                }

                //receive block, insert block and broadcast block hashes
                Message::Blocks(nonce) => {
                    let mut blockchain = self.blockchain.lock().unwrap();
                    let blocks = nonce.clone();
                    let mut orph_buff = self.orph_buff.lock().unwrap();
                    let mut new_blocks: Vec<H256> = Vec::new();
                    let mut trans_memopool = self.trans_memopool.lock().unwrap();
                    let mut state = self.state.lock().unwrap();
                    for block in blocks { 
                        let mut hash = block.hash();
                        let mut block = block.clone();
                        if hash > block.header.difficulty { // PoW check
                            continue;
                        }
                        if blockchain.block_map.contains_key(&hash) { //already in blockchain
                            continue;
                        }
                        new_blocks.push(hash);
                        // check parent
                        let mut p_hash = block.header.parent;
                        if !blockchain.block_map.contains_key(&p_hash) { // parent not in chain
                            if !orph_buff.contains_key(&p_hash) {
                                orph_buff.insert(p_hash, block);
                            }
                        }else{ // parent in the chain                               
                            let mut p_block = blockchain.block_map[&p_hash].clone();
                            let mut p_diff = p_block.header.difficulty;
                            let mut diff = block.header.difficulty;
                            let content = block.content.content.clone();
                            // println!("***************** {:?}", content.len());
                            let mut removeContent = false;
                            loop{
                                if hash < diff && diff == p_diff{ //PoW check    
                                    removeContent = true; 
                                    state.update(&block);           
                                    blockchain.insert(&block);
                                    if orph_buff.contains_key(&hash) { 
                                        let orph_block = orph_buff.remove(&hash).unwrap();
                                        block = orph_block.clone();
                                        hash = orph_block.hash();  
                                        p_hash = block.header.parent; 
                                        p_block = blockchain.block_map[&p_hash].clone();   
                                        p_diff = p_block.header.difficulty;       
                                        diff = block.header.difficulty;             
                                    } else {
                                        break; //exit loop
                                    }    
                                } else {
                                    break; //exit loop
                                }
                            }  
                            if removeContent {
                                for trans in content {
                                    let trans_hash = trans.hash();
                                    trans_memopool.trans_map.remove(&trans_hash);
                                }
                            }                  
                        }
                    }
                    let mut buffered_block_hashs: Vec<H256> = Vec::new();
                    for (orph_p_hash, orph_block) in orph_buff.clone() {
                        buffered_block_hashs.push(orph_p_hash);
                    }
                    if buffered_block_hashs.len() != 0 {
                        self.server.broadcast(Message::GetBlocks(buffered_block_hashs));
                    }
                    if new_blocks.len() != 0 {
                        self.server.broadcast(Message::NewBlockHashes(new_blocks));
                    }
                }
                // receive transaction hashes and find transaction hash not in mempool 
                Message::NewTransactionHashes(vec_transaction_hashs) => {
                    // println!("NewTransactionHashes");
                    let mut trans_memopool = self.trans_memopool.lock().unwrap();
                    let mut vec_hash: Vec<H256> = Vec::new();
                    for trans_hash in vec_transaction_hashs {
                        if trans_memopool.trans_map.get(&trans_hash).is_none() {
                            vec_hash.push(trans_hash.clone());
                        }
                    }
                    if vec_hash.len() > 0 {
                        peer.write(Message::GetTransactions(vec_hash));
                    }
                }
                // receive transaction hashes and reply trans
                Message::GetTransactions(vec_transaction_hashs) => {
                    // println!("GetTransactions");
                    let mut trans_memopool = self.trans_memopool.lock().unwrap();
                    let mut vec_trans: Vec<SignedTransaction> = Vec::new();
                    for trans_hash in vec_transaction_hashs {
                        if !trans_memopool.trans_map.get(&trans_hash).is_none() {
                            vec_trans.push(trans_memopool.trans_map.get(&trans_hash).unwrap().clone());
                        }
                    }
                    if vec_trans.len() > 0 {
                        peer.write(Message::Transactions(vec_trans));
                    }
                }
                //receive transaction, insert transcation and broadcast inserted transaction hashes
                Message::Transactions(vec_transactions) => {
                    // println!("Transactions");
                    let mut trans_memopool = self.trans_memopool.lock().unwrap();
                    let mut state = self.state.lock().unwrap();
                    let mut vec_hash: Vec<H256> = Vec::new();
                    for trans in vec_transactions {
                        let check_result = verify(&trans.transaction, &trans.public_key, &trans.signature, &state);
                        if check_result {
                            let trans_hash = trans.hash();
                            if trans_memopool.trans_map.get(&trans_hash).is_none() {
                                trans_memopool.trans_map.insert(trans_hash, trans.clone());
                                // println!("1 {:?}", trans_memopool.trans_map.len());
                                vec_hash.push(trans_hash);
                            }
                        }
                    }
                    if vec_hash.len() > 0 {
                        self.server.broadcast(Message::NewTransactionHashes(vec_hash));
                    }
                }
                _ => unimplemented!()
               
            }
        }
    }
}

#[cfg(any(test,test_utilities))]
struct TestMsgSender {
    s: smol::channel::Sender<(Vec<u8>, peer::Handle)>
}
#[cfg(any(test,test_utilities))]
impl TestMsgSender {
    fn new() -> (TestMsgSender, smol::channel::Receiver<(Vec<u8>, peer::Handle)>) {
        let (s,r) = smol::channel::unbounded();
        (TestMsgSender {s}, r)
    }

    fn send(&self, msg: Message) -> PeerTestReceiver {
        let bytes = bincode::serialize(&msg).unwrap();
        let (handle, r) = peer::Handle::test_handle();
        smol::block_on(self.s.send((bytes, handle))).unwrap();
        r
    }
}
// #[cfg(any(test,test_utilities))]
// /// returns two structs used by tests, and an ordered vector of hashes of all blocks in the blockchain
// fn generate_test_worker_and_start() -> (TestMsgSender, ServerTestReceiver, Vec<H256>) {
//     let (server, server_receiver) = ServerHandle::new_for_test();
//     let (test_msg_sender, msg_chan) = TestMsgSender::new();
    
//     let blockchain = Blockchain::new();
//     let tip = blockchain.tip();
//     let blockchain_arc = &Arc::new(Mutex::new(blockchain));
//     let orph_buff = &Arc::new(Mutex::new(HashMap::new())); //add
//     let worker = Worker::new(1, msg_chan, &server, blockchain_arc, orph_buff); //add
//     worker.start(); 
//     (test_msg_sender, server_receiver, vec![tip])
// }

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

// #[cfg(test)]
// mod test {
//     use ntest::timeout;
//     use crate::types::block::generate_random_block;
//     use crate::types::hash::Hashable;

//     use super::super::message::Message;
//     use super::generate_test_worker_and_start;

//     #[test]
//     #[timeout(60000)]
//     fn reply_new_block_hashes() {
//         let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
//         let random_block = generate_random_block(v.last().unwrap());
//         let mut peer_receiver = test_msg_sender.send(Message::NewBlockHashes(vec![random_block.hash()]));
//         let reply = peer_receiver.recv();
//         if let Message::GetBlocks(v) = reply {
//             assert_eq!(v, vec![random_block.hash()]);
//         } else {
//             panic!();
//         }
//     }
//     #[test]
//     #[timeout(60000)]
//     fn reply_get_blocks() {
//         let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
//         let h = v.last().unwrap().clone();
//         let mut peer_receiver = test_msg_sender.send(Message::GetBlocks(vec![h.clone()]));
//         let reply = peer_receiver.recv();
//         if let Message::Blocks(v) = reply {
//             assert_eq!(1, v.len());
//             assert_eq!(h, v[0].hash())
//         } else {
//             panic!();
//         }
//     }
//     #[test]
//     #[timeout(60000)]
//     fn reply_blocks() {
//         let (test_msg_sender, server_receiver, v) = generate_test_worker_and_start();
//         let random_block = generate_random_block(v.last().unwrap());
//         let mut _peer_receiver = test_msg_sender.send(Message::Blocks(vec![random_block.clone()]));
//         let reply = server_receiver.recv().unwrap();
//         if let Message::NewBlockHashes(v) = reply {
//             assert_eq!(v, vec![random_block.hash()]);
//         } else {
//             panic!();
//         }
//     }
// }

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST