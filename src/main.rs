pub mod data;
pub mod discovery;
pub mod flight_computer;

use std::thread;
use data::DataReceiver;
use flight_computer::state;

fn main() {
    let mut data_receiver = DataReceiver::new();
    let mut fc_state = state::State::Init;
    let mut fc_state_data = state::Data::new();

    let data_thread = thread::spawn(move || loop {
        let _ = data_receiver.receive();
    });

    let state_thread = thread::spawn(move || loop {
        fc_state = fc_state.next(&mut fc_state_data);
    });

    data_thread.join().unwrap();
    state_thread.join().unwrap();
}
