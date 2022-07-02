pub mod tx_generator;

use log::info;

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;

use std::thread;

use crate::network::message::Message;
use crate::types::address::Address;
use crate::types::block::Block;
use crate::blockchain::Blockchain;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::types::merkle::MerkleTree;
use crate::types::block::*;
use crate::types::transaction::*;
use crate::network::server::Handle as ServerHandle;
use crate::types::key_pair;
use ring::signature::KeyPair;
use std::{collections::{HashMap, HashSet}, ops::Add};
use rand::{thread_rng, Rng};


// use rand::Rng;
use crate::types::hash::Hashable;
use ring::signature::Ed25519KeyPair;


enum ControlSignal {
    Start(u64), // the number controls the theta of interval between transaction generation
    Update, // update the transaction generator, it may due to new transaction
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct TXGenerator {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    server: ServerHandle,
    state: Arc<Mutex<State>>,
    key: Ed25519KeyPair,
    address: Address
}

#[derive(Clone)]
pub struct GeneratorHandle {
    /// Channel for sending signal to the transaction generator thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(server: &ServerHandle, state: &Arc<Mutex<State>>, addr: Address) -> (TXGenerator, GeneratorHandle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let key = key_pair::random();
    let generator = TXGenerator {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        state: Arc::clone(state),
        key: key,
        address: addr,
    };
   

    let handle = GeneratorHandle {
        control_chan: signal_chan_sender,
    };

    (generator, handle)
}

// #[cfg(any(test,test_utilities))]
// fn test_new() -> (Context, Handle, Receiver<Block>) {
//     new(&Arc::new(Mutex::new(Blockchain::new())))
// }

impl GeneratorHandle {
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

impl TXGenerator {

    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.generator_loop();
            })
            .unwrap();
        info!("Generator initialized into paused mode");
    }

    fn generator_loop(&mut self) {
        // main transaction generator loop
        let mut sent_created_account = false;
        let mut current_nonce = 0;
        let mut other_accounts = Vec::new();
        let mut other_accounts_hashSet = HashSet::new();
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    match signal {
                        ControlSignal::Exit => {
                            info!("Generator shutting down");
                            self.operating_state = OperatingState::ShutDown;
                        }
                        ControlSignal::Start(i) => {
                            info!("Generator starting in continuous mode with theta {}", i);
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
                                info!("Generator shutting down");
                                self.operating_state = OperatingState::ShutDown;
                            }
                            ControlSignal::Start(i) => {
                                info!("Generator starting in continuous mode with theta {}", i);
                                self.operating_state = OperatingState::Run(i);
                            }
                            ControlSignal::Update => {
                                unimplemented!()
                            }
                        };
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Generator control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            // println!("Generator is running");
            let state = self.state.lock().unwrap();
            if sent_created_account == false {
                sent_created_account = true;
                let account = self.address;
                let signedTransaction = generate_random_signed_transaction(account, account, 1000, 0, &self.key);
                self.server.broadcast(Message::Transactions(vec![signedTransaction]));
                continue;
            }
            if state.accounts.len() > 1 {
                if state.accounts.len() > other_accounts.len() + 1 {
                    for (account, _) in &state.accounts {
                        let account = *account;
                        if !other_accounts_hashSet.contains(&account) && account != self.address {
                            other_accounts_hashSet.insert(account);
                            other_accounts.push(account);
                        }
                    }
                }
                let account_info = match state.accounts.get(&self.address) {
                    Some(v) => v,
                    None => {
                        return;
                    }
                };
                let account_info = *account_info;
                if current_nonce == account_info.0 {
                    current_nonce = account_info.0 + 1;
                    let nonce = current_nonce;

                    let old_value = account_info.1;
                    if old_value <= 3 {
                        continue;
                    }
                    let mut rng = rand::thread_rng();

                    let rand_index = rng.gen_range(0..other_accounts.len());
                    let receiver = other_accounts[rand_index];
                    let value = rng.gen_range(1..old_value/2);
                    let signedTransaction = generate_random_signed_transaction(self.address, receiver, value, nonce, &self.key);
                    self.server.broadcast(Message::Transactions(vec![signedTransaction]));
                    // println!("SENT from {:?} nonce {:?}", self.address, nonce);
                }
                
                
            }
            
            // let signedTransaction = generate_random_signed_transaction();

            // self.server.broadcast(Message::Transactions(vec![signedTransaction]));



            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    // println!("theta is {:?}", i);
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}