use super::message::Message;
use super::peer;
use super::server::Handle as ServerHandle;
use crate::blockchain::Blockchain;
use crate::types::block::Block;
use crate::types::hash::{H256, Hashable};

use log::{debug, warn, error};

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
}


impl Worker {
    pub fn new(
        num_worker: usize,
        msg_src: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
        server: &ServerHandle,
        blockchain: &Arc<Mutex<Blockchain>>, //change
    ) -> Self {
        Self {
            msg_chan: msg_src,
            num_worker,
            server: server.clone(),
            blockchain: Arc::clone(blockchain), //change
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

                // find hashes not in blockchain
                Message::NewBlockHashes(nonce) =>{
                    // debug!("NewBlockHashes: {:?}", nonce);
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
                        // for i in nonce.iter(){ //find hash not in
                        //     if self.blockchain.lock().unwrap().block_map.get(&i).is_none(){
                        //         vec_hash.push(*i);
                        //         // peer.write(Message::GetBlocks(nonce));
                        //     }
                        // }
                        // if vec_hash.len() != 0{
                            // println!("We got here first: {:?}", vec.len());
                            peer.write(Message::GetBlocks(vec_hash));
                        // }
                    }
                }
                //receive hashes and reply Blocks.
                Message::GetBlocks(blk) => {
                    // debug!("GetBlocks: {:?}", blk);
                    let blockchain = self.blockchain.lock().unwrap();
                    let mut nonce = blk.clone();
                    let mut vec = Vec::new();
                    for i in 0..nonce.len() {
                        let hash = nonce[i];
                        if !blockchain.block_map.get(&hash).is_none() {
                            // let blk = blockchain.block_map.get(&hash).unwrap().clone();
                            vec.push(blockchain.block_map.get(&hash).unwrap().clone());
                        }
                    }
                    if vec.len() != 0{
                        //println!("We got here: {:?}", vec.len());
                        peer.write(Message::Blocks(vec));
                    }
                        // for i in nonce.iter(){
                        //     if !blockchain.block_map.get(&i).is_none(){
                        //         // let temp = self.blockchain.lock().unwrap().map.get(&i).unwrap();
                        //         // peer.write(Message::GetBlocks(nonce));
                        //         vec.push(blockchain.block_map.get(&i).unwrap().clone());
                        //     }
                        // }
                    // }
                }
                //insert block and broadcast block hashes
                Message::Blocks(blk) => {
                    // debug!("Blocks: {:?}", blk);
                    let mut blockchain = self.blockchain.lock().unwrap();
                    let mut nonce = blk.clone();
                    // let mut vec_hash: Vec<H256> = Vec::new();
                    for i in 0..nonce.len(){
                        let blk_copy = &nonce[i];
                        let hash =blk_copy.hash();
                        if blockchain.block_map.get(&hash).is_none(){
                            blockchain.insert(&blk_copy);
                            // vec_hash.push(hash);
                            // self.server.broadcast(Message::NewBlockHashes(vec_hash));
                            self.server.broadcast(Message::NewBlockHashes(vec![hash]));
                        }
                    }
                    // for i in nonce.iter(){ //iter?
                    //     if blockchain.block_map.get(&i.hash()).is_none(){
                    //         blockchain.insert(&i);
                    //         vec_hash.push(hash);
                    //         self.server.broadcast(Message::NewBlockHashes(vec_hash));
                    //     }
                    // }
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
#[cfg(any(test,test_utilities))]
/// returns two structs used by tests, and an ordered vector of hashes of all blocks in the blockchain
fn generate_test_worker_and_start() -> (TestMsgSender, ServerTestReceiver, Vec<H256>) {
    let (server, server_receiver) = ServerHandle::new_for_test();
    let (test_msg_sender, msg_chan) = TestMsgSender::new();
    
    let blockchain = Blockchain::new();
    let tip = blockchain.tip();
    let blockchain_arc = &Arc::new(Mutex::new(blockchain));
    let worker = Worker::new(1, msg_chan, &server, blockchain_arc);
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