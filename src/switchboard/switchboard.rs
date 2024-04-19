use std::{collections::HashMap, net::{SocketAddr, UdpSocket}, sync::{mpsc::Sender, Arc, RwLock}};
use common::comm::{BoardId, DataMessage, DataPoint};
use jeflog::{fail, pass, warn};
use crate::{handler, state::SharedState, TuiSender, FC_BOARD_ID};

/// Wakes when there's something to be passed along. Think of it like a telephone operator.
pub fn switchboard(shared: SharedState, snooze: Sender<BoardId>, gig: Sender<(BoardId, Vec<DataPoint>)>, handshake_sender: UdpSocket, reciever: UdpSocket, sockets: Arc<RwLock<HashMap<BoardId, SocketAddr>>>, tui_tx: TuiSender) -> impl FnOnce() -> () {
  move || {
    let mut buffer = [0; crate::DATA_MESSAGE_BUFFER_SIZE];

    loop {
      // Move the incoming UDP data into a buffer
      let (message_length, sender_address) = match reciever.recv_from(&mut buffer) {
        Ok(data) => data,
        Err(e) => {
          fail!("Couldn't insert data into switchboard buffer, aborting..: {e}");
          handler::abort(&shared);
          continue;
        }
      };

      // Interpret the data in the buffer
      let incoming_data = match postcard::from_bytes::<DataMessage>(&buffer[..message_length]) {
        Ok(data) => data,
        Err(e) => {
          fail!("postcard couldn't interpret the buffer data, ignoring...: {e}");
          continue;
        }
      };

      let board_id = match incoming_data {
        DataMessage::Identity(board_id) => {
          let mut sockets = sockets.write().unwrap();
          sockets.insert(board_id.clone(), sender_address);

          pass!("Recieved identity message from board {board_id}");
					
					let identity = DataMessage::Identity(String::from(FC_BOARD_ID));

					let handshake = match postcard::to_slice(&identity, &mut buffer) {
						Ok(identity) => identity,
						Err(e) => {
							warn!("postcard returned this error when attempting to serialize DataMessage::Identity: {e}");
							continue;
						}
					};

					if let Err(e) = handshake_sender.send_to(handshake, sender_address) {
						fail!("Couldn't send DataMessage::Identity to ip {sender_address}: {e}");
					} else {
						pass!("Sent DataMessage::Identity to {sender_address} successfully.");
					}

          //if let Err(e) = tui_tx.send(TuiMessage::Identity(board_id.clone())) {
          //  fail!("Couldn't send message to TUI. tui_rx might've been dropped: {e}");
          //}

          board_id
        },
        DataMessage::Sam(board_id, datapoints) => {
          if let Err(e) = gig.send((board_id.clone(), datapoints.to_vec())) {
            fail!("Worker unexpectedly dropped the receiving end of the gig channel ({e}). Aborting and committing suicide...");
            handler::abort(&shared);
            break;
          }

          board_id
        },
        DataMessage::Bms(board_id) => board_id,
        DataMessage::FlightHeartbeat => {
          warn!("Recieved a FlightHeartbeat from {sender_address}. This shouldn't happen, ignoring...");
          continue;
        }
      };

      if let Err(e) = snooze.send(board_id) {
        fail!("Lifetime unexpectedly dropped the receiving end of the snooze channel ({e}). Aborting and committing suicide...");
        handler::abort(&shared);
        break;
      }
    }
  }
}