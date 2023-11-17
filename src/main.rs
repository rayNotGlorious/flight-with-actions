pub mod data;
pub mod discovery;
pub mod flight_computer;
pub mod state;

use std::{thread, sync::{RwLock, Arc}};
use data::DataReceiver;

fn main() {
    let state: state::State = state::State::new();

    let state_lock = Arc::new(RwLock::new(state));




    
    let mut data_receiver = DataReceiver::new(Arc::clone(&state_lock));
    let mut fc_state = flight_computer::State::Init;
    let mut fc_state_data = flight_computer::Data::new();

    let data_thread = thread::spawn(move || loop {
        let _ = data_receiver.receive();
    });

    let state_thread = thread::spawn(move || loop {
        fc_state = fc_state.next(&mut fc_state_data);
    });

    data_thread.join().unwrap();
    state_thread.join().unwrap();
}
