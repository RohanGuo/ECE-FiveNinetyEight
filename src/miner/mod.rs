pub mod worker;

use log::info;

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;

use std::thread;

use crate::types::block::Block;
use crate::blockchain::Blockchain;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::types::merkle::MerkleTree;
use crate::types::block::*;
use crate::types::transaction::*;


use rand::Rng;
use crate::types::hash::Hashable;


enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Update, // update the block in mining, it may due to new blockchain tip or new transaction
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    finished_block_chan: Sender<Block>,
    blockchain: Arc<Mutex<Blockchain>>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(blockchain: &Arc<Mutex<Blockchain>>) -> (Context, Handle, Receiver<Block>) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let (finished_block_sender, finished_block_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        finished_block_chan: finished_block_sender,
        blockchain: Arc::clone(blockchain),
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle, finished_block_receiver)
}

#[cfg(any(test,test_utilities))]
fn test_new() -> (Context, Handle, Receiver<Block>) {
    new(&Arc::new(Mutex::new(Blockchain::new())))
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
            .unwrap();
    }

    pub fn update(&self) {
        self.control_chan.send(ControlSignal::Update).unwrap();
    }
}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn miner_loop(&mut self) {
        // main mining loop
        //let locked_parent = self.blockchain.lock().unwrap();
        //let mut parent = locked_parent.tip();
        //let difficulty =locked_parent.block_map[&parent].header.difficulty;
        
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    match signal {
                        ControlSignal::Exit => {
                            info!("Miner shutting down");
                            self.operating_state = OperatingState::ShutDown;
                        }
                        ControlSignal::Start(i) => {
                            info!("Miner starting in continuous mode with lambda {}", i);
                            self.operating_state = OperatingState::Run(i);
                        }
                        ControlSignal::Update => {
                            // in paused state, don't need to update
                        }
                    };
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        match signal {
                            ControlSignal::Exit => {
                                info!("Miner shutting down");
                                self.operating_state = OperatingState::ShutDown;
                            }
                            ControlSignal::Start(i) => {
                                info!("Miner starting in continuous mode with lambda {}", i);
                                self.operating_state = OperatingState::Run(i);
                            }
                            ControlSignal::Update => {
                                unimplemented!()
                            }
                        };
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }
            // TODO for student: actual mining, create a block
            // TODO for student: if block mining finished, you can have something like: self.finished_block_chan.send(block.clone()).expect("Send finished block error");
            
            let locked_parent = self.blockchain.lock().unwrap();
            let mut parent = locked_parent.tip();
            let difficulty =locked_parent.block_map[&parent].header.difficulty;
            // println!("{:?}", parent);

            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
            let signed_transactions:Vec<SignedTransaction> = Vec::new();
            let merkle_tree = MerkleTree::new(&signed_transactions);
            let merkle_root = merkle_tree.root();
            let mut rng = rand::thread_rng();
            let nonce = rng.gen();
            
            let header = Header{
                parent: parent,
                nonce: nonce,
                difficulty: difficulty,
                timestamp: timestamp,
                merkle_root: merkle_root,
            };
            
            let content = Content{
                content: vec![],
            };

            let block = Block{
                header: header,
                content: content,
            };

            if block.hash() <= difficulty {
                // println!("parent  1 {:?}", block.header.parent);
                self.finished_block_chan.send(block.clone()).expect("Send finished block error");
                //self.blockchain.lock().unwrap().insert(&block.clone());
                // parent = block.hash();
                //println!("miner {}",block.hash());
            }

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use ntest::timeout;
    use crate::types::hash::Hashable;

    #[test]
    #[timeout(30000)]
    fn miner_three_block() {
        let (miner_ctx, miner_handle, finished_block_chan) = super::test_new();
        //let worker = crate::miner::worker::Worker::new(finished_block_chan,&miner_ctx.blockchain);
        //worker.start();
        miner_ctx.start();
        //println!("111");
        miner_handle.start(0);

        //println!("222");
        let mut block_prev = finished_block_chan.recv().unwrap();
        //println!("333");

        for _ in 0..2 {
            let block_next = finished_block_chan.recv().unwrap();
            //println!("prev {}",block_prev.hash());
            //println!("next {}",block_next.hash());
            assert_eq!(block_prev.hash(), block_next.get_parent());
            block_prev = block_next;
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST