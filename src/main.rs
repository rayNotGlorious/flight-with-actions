pub mod data;
pub mod discovery;
pub mod flight_computer;
pub mod state;

use std::{thread, collections::HashMap, sync::{RwLock, Arc}};
use data::DataReceiver;

use fs_protobuf_rust::compiled::mcfs::board;

fn main() {
    let mut state: state::State = state::State {
        sensor_data: HashMap::new(),
        channel_mapping: HashMap::new(),
    };

    let state_lock = Arc::new(RwLock::new(state));

    state_lock.write().unwrap().channel_mapping.insert(
        state::ChannelIdentifier::new(1,  board::ChannelType::CURRENT_LOOP as u32, 1),
        "PT1".to_string(),
    );

    state_lock.write().unwrap().channel_mapping.insert(
        state::ChannelIdentifier::new(1,  board::ChannelType::CURRENT_LOOP as u32, 1),
        "PT2".to_string(),
    );

    state_lock.write().unwrap().channel_mapping.insert(
        state::ChannelIdentifier::new(1,  board::ChannelType::CURRENT_LOOP as u32, 1),
        "PT3".to_string(),
    );




    
    // let mut data_receiver = DataReceiver::new(Arc::clone(&state_lock)));
    // let mut fc_state = flight_computer::State::Init;
    // let mut fc_state_data = flight_computer::Data::new();

    // let data_thread = thread::spawn(move || loop {
    //     let _ = data_receiver.receive();
    // });

    // let state_thread = thread::spawn(move || loop {
    //     fc_state = fc_state.next(&mut fc_state_data);
    // });

    // data_thread.join().unwrap();
    // state_thread.join().unwrap();
}
