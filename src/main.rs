pub mod data_receiver;
pub mod discovery;
pub mod flight_computer;
pub mod state;
pub mod sequences;

use std::{thread, sync::{RwLock, Arc}};
use data_receiver::DataReceiver;
use flight_computer::FlightComputer;

fn main() {
    let state_lock = Arc::new(RwLock::new(state::State::new()));
    
    let mut flight_computer = FlightComputer::new(Arc::clone(&state_lock));
    let mut data_receiver = DataReceiver::new(Arc::clone(&state_lock));

    let state_thread = thread::spawn(move || flight_computer.run() );
    let data_thread = thread::spawn(move || loop {let _ = data_receiver.receive();});

    data_thread.join().unwrap();
    state_thread.join().unwrap();
}
