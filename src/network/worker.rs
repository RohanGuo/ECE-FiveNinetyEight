use super::message::Message;
use super::peer;
use super::server::Handle as ServerHandle;
use crate::blockchain::Blockchain;
use crate::types::block::Block;
use crate::types::hash::{H256, Hashable};

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
    blockchain: Arc<Mutex<Blockchain>>, //change
    orph_buff: Arc<Mutex<HashMap<H256, Block>>>, //add
}


impl Worker {
    pub fn new(
        num_worker: usize,
        msg_src: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
        server: &ServerHandle,
        blockchain: &Arc<Mutex<Blockchain>>, //change
        orph_buff: &Arc<Mutex<HashMap<H256, Block>>>, //add
    ) -> Self {
        Self {
            msg_chan: msg_src,
            num_worker,
            server: server.clone(),
            blockchain: Arc::clone(blockchain), //change
            orph_buff: Arc::clone(orph_buff), //add
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
                    //debug!("NewBlockHashes: {:?}", nonce);
                    if nonce.len() != 0{
                        let mut vec_hash: Vec<H256> = Vec::new();
                        let blockchain = self.blockchain.lock().unwrap();
                        // for i in 0..nonce.len(){
                        for hash in nonce.clone() {   
                            // let hash = nonce[i];
                            if blockchain.block_map.get(&hash).is_none(){
                                vec_hash.push(hash);
                                // peer.write(Message::GetBlocks(nonce));
                            }
                        }
                        peer.write(Message::GetBlocks(vec_hash));
                    }
                }
                //receive hashes and reply Blocks.
                Message::GetBlocks(nonce) => {
                    //debug!("GetBlocks: {:?}", nonce);
                    let blockchain = self.blockchain.lock().unwrap();
                    let mut blocks = nonce.clone();
                    let mut vec = Vec::new();
                    for i in 0..blocks.len() {
                        let hash = blocks[i];
                        if !blockchain.block_map.get(&hash).is_none() {
                            // let blk = blockchain.block_map.get(&hash).unwrap().clone();
                            vec.push(blockchain.block_map.get(&hash).unwrap().clone());
                        }
                    }
                    if vec.len() != 0{
                        //println!("We got here: {:?}", vec.len());
                        peer.write(Message::Blocks(vec));
                    }
                }

                //receive block, insert block and broadcast block hashes
                Message::Blocks(nonce) => {
                    //debug!("Blocks: {:?}", nonce);
                    let mut blockchain = self.blockchain.lock().unwrap();
                    let blocks = nonce.clone();
                    let mut orph_buff = self.orph_buff.lock().unwrap();

                    for block in blocks {
                        let mut hash = block.hash();
                        let mut block = block.clone();
                        if !blockchain.block_map.contains_key(&hash){ //not in blockchain, check parent

                            let mut p_hash = block.header.parent;
                            // println!("{:?}", p_hash);
                            if !blockchain.block_map.contains_key(&p_hash) { //parent not in chain
                                // println!("0");
                                orph_buff.insert(p_hash, block);
                                peer.write(Message::GetBlocks(vec![p_hash]));

                            }else{ //parent in chain
                                
                                let mut p_block = blockchain.block_map[&p_hash].clone();
                                let mut p_diff = p_block.header.difficulty;
                                let mut diff = block.header.difficulty;
                                loop{
                                    if hash < diff && diff == p_diff{ //PoW check
                                        
                                        blockchain.insert(&block);
                                        self.server.broadcast(Message::NewBlockHashes(vec![hash]));
                                        // println!("111");
                                        if orph_buff.contains_key(&hash) { 
                                            //inserted block is parent in orph buff  
                                            // println!("222");                                                                                   
                                            let orph_block = orph_buff.remove(&hash).unwrap();
                                            block = orph_block.clone();
                                            hash = orph_block.hash();  
                                            p_hash = block.header.parent; 
                                            p_block = blockchain.block_map[&p_hash].clone();   
                                            p_diff = p_block.header.difficulty;       
                                            diff = block.header.difficulty;             
                                        }else{
                                            break; //exit loop
                                        }    
                                    }else{
                                        break; //exit loop
                                    }
                                }
                                
                            }
                        }
                    }
                }
                _ => unimplemented!()
                // loop {
                //     // received block is parent of block in orph_buff
                //     if orph_buff.contains_key(&hash) { 
                //         let orph_block = orph_buff.remove(&hash).unwrap();
                //         blockchain.insert(&orph_block);
                //         self.server.broadcast(Message::NewBlockHashes(vec![orph_block.hash()]));
                        
                //         hash = orph_block.hash();
                //     }
                //     else {
                //         break;
                //     }
                // }

                    // // let mut vec_hash: Vec<H256> = Vec::new();
                    // while !nonce.is_empty(){
                    //     let blk_copy = nonce.remove(0);
                    //     let blk_hash =blk_copy.hash();
                    //     let parent_h = blk_copy.header.parent;
                    //     let parent_option_block = blockchain.block_map.get(&blk_hash);
                    //     match parent_option_block {
                    //         Some(parent_b) => {
                    //             let diff = parent_b.header.difficulty;
                    //             if blk_hash < diff {
                    //                 blockchain.insert(&blk_copy);
                    //                 self.server.broadcast(Message::NewBlockHashes(vec![blk_hash]));
                    //             }else{
                    //                 continue;
                    //             }  
                    //         }
                    //         None => {
                    //             orph_buff.push(blk_copy);
                    //             peer.write(Message::GetBlocks(vec![parent_h]));
                    //         }
                    //     }
                    //     for i in 0..self.orph_buff.len(){ 
                    //         if orph_buff[i].header.parent == blk_hash{
                    //             nonce.push(orph_buff[i].clone());
                    //             orph_buff.remove(i);
                    //             break;
                    //         }
                    //     }
                    // }
                        // if blockchain.block_map.get(&blk_hash).is_none(){ //block not in blockchain
                        //     if blockchain.block_map.get(&parent_h).is_none(){ //parent not in blockchain
                        //         self.orph_buff.push(blk_copy);
                        //         peer.write(Message::GetBlocks(vec![parent_h]));
                        //     }else{ // parent in blockchain
                        //         let parent_b = blockchain.block_map.get(&parent_h);
                        //         let diff = parent_b.
                        //         if blk_hash < diff {
                        //             blockchain.insert(&blk_copy);
                        //             self.server.broadcast(Message::NewBlockHashes(vec![hash]));
                        //         }else{
                        //             continue;
                        //         }       
                        //     }
                        // }
                        //processed block is a parent to any block in the orphan buffer
                    // for i in 0..nonce.len(){
                    //     let blk_copy = &nonce[i];
                    //     let hash =blk_copy.hash();
                    //     if blockchain.block_map.get(&hash).is_none(){ //block not in blockchain
                    //         // check parents
                    //         let p_h = blk_copy.header.parent; //parent hash
                    //         if blockchain.block_map.get(&p_h).is_none(){ //parent not in blockchain
                    //             self.orph_buff.insert(hash, *blk_copy);
                    //             peer.write(Message::GetBlocks(vec![p_h]));
                    //         }else{ // parent in blockchain
                    //             blockchain.insert(&blk_copy);
                    //             self.server.broadcast(Message::NewBlockHashes(vec![hash]));
                    //         }
                    //     }
                    //     if !self.orph_buff.get(&hash).is_none(){

                    //     }
                    //     for i in self.orph_buff.iter(){
                    //         if i.header.parent == hash
                    //             orph_buff.remove(k)
                    //             nonce.add()
                    //     }
                    //}
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
#[cfg(any(test,test_utilities))]
/// returns two structs used by tests, and an ordered vector of hashes of all blocks in the blockchain
fn generate_test_worker_and_start() -> (TestMsgSender, ServerTestReceiver, Vec<H256>) {
    let (server, server_receiver) = ServerHandle::new_for_test();
    let (test_msg_sender, msg_chan) = TestMsgSender::new();
    
    let blockchain = Blockchain::new();
    let tip = blockchain.tip();
    let blockchain_arc = &Arc::new(Mutex::new(blockchain));
    let orph_buff = &Arc::new(Mutex::new(HashMap::new())); //add
    let worker = Worker::new(1, msg_chan, &server, blockchain_arc, orph_buff); //add
    worker.start(); 
    (test_msg_sender, server_receiver, vec![tip])
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use ntest::timeout;
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;

    use super::super::message::Message;
    use super::generate_test_worker_and_start;

    #[test]
    #[timeout(60000)]
    fn reply_new_block_hashes() {
        let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
        let random_block = generate_random_block(v.last().unwrap());
        let mut peer_receiver = test_msg_sender.send(Message::NewBlockHashes(vec![random_block.hash()]));
        let reply = peer_receiver.recv();
        if let Message::GetBlocks(v) = reply {
            assert_eq!(v, vec![random_block.hash()]);
        } else {
            panic!();
        }
    }
    #[test]
    #[timeout(60000)]
    fn reply_get_blocks() {
        let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
        let h = v.last().unwrap().clone();
        let mut peer_receiver = test_msg_sender.send(Message::GetBlocks(vec![h.clone()]));
        let reply = peer_receiver.recv();
        if let Message::Blocks(v) = reply {
            assert_eq!(1, v.len());
            assert_eq!(h, v[0].hash())
        } else {
            panic!();
        }
    }
    #[test]
    #[timeout(60000)]
    fn reply_blocks() {
        let (test_msg_sender, server_receiver, v) = generate_test_worker_and_start();
        let random_block = generate_random_block(v.last().unwrap());
        let mut _peer_receiver = test_msg_sender.send(Message::Blocks(vec![random_block.clone()]));
        let reply = server_receiver.recv().unwrap();
        if let Message::NewBlockHashes(v) = reply {
            assert_eq!(v, vec![random_block.hash()]);
        } else {
            panic!();
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST