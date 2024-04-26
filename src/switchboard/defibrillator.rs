use std::{collections::{HashMap, HashSet}, net::{SocketAddr, UdpSocket}, sync::{Arc, Mutex, RwLock}, thread};
use common::comm::{BoardId, DataMessage};
use jeflog::fail;
use crate::{handler, state::SharedState, HEARTBEAT_PERIOD};

/// Wakes every HEARTBEAT_RATE to send heartbeats to all the connected Sam boards to ensure that the FC isn't disconnected.
pub fn defibrillator(shared: SharedState, sender: UdpSocket, sockets: Arc<RwLock<HashMap<BoardId, SocketAddr>>>, statuses: Arc<Mutex<HashSet<BoardId>>>) -> impl FnOnce() -> () {
  move || {
    let mut buf = vec![0; crate::HEARTBEAT_BUFFER_SIZE];

    let heartbeat = match postcard::to_slice(&DataMessage::FlightHeartbeat, &mut buf) {
      Ok(package) => package,
      Err(e) => {
        fail!("postcard returned this error when attempting to serialize DataMessage::FlightHeartbeat: {e}");
        handler::abort(&shared);
        return;
      }
    };
    
    loop {
      thread::sleep(HEARTBEAT_PERIOD);

      let sockets = sockets.read().unwrap();
      let statuses = statuses.lock().unwrap();
      let mut abort = false;
      for (board_id, address) in sockets.iter() {
        if !statuses.contains(board_id) {
          continue;
        }

        if let Err(e) = sender.send_to(heartbeat, address) {
          fail!("Couldn't send heartbeat to address {address:#?}: {e}");
          abort = true;
        }
      }

      if abort {
        fail!("Aborting...");
        handler::abort(&shared);
      }
    }
  }
}