mod switchboard;
mod worker;
mod lifetime;
mod defibrillator;
mod commander;

use switchboard::switchboard;
use lifetime::lifetime;
use worker::worker;
use defibrillator::defibrillator;
use commander::commander;
use std::{collections::{HashMap, HashSet}, io, net::UdpSocket, sync::{mpsc, Arc, Mutex, RwLock}, thread};
use crate::{state::SharedState, CommandSender};

// Concerns: might be a bit too abort happy?

/// one-shot function that starts the switchboard.
pub fn start(shared: SharedState, socket: UdpSocket) -> io::Result<CommandSender> {
  let reciever = socket.try_clone()?;
  let sender = socket.try_clone()?;
  let command_sender = socket.try_clone()?;

  let (snooze_tx, snooze_rx) = mpsc::channel();
  let (gig_tx, gig_rx) = mpsc::channel();
  let (command_tx, command_rx) = mpsc::channel();

  let statuses = Arc::new(Mutex::new(HashSet::new()));
  let sockets = Arc::new(RwLock::new(HashMap::new()));
  
  thread::spawn(switchboard(shared.clone(), snooze_tx, gig_tx, socket, reciever, sockets.clone()));
  thread::spawn(lifetime(shared.clone(), snooze_rx, statuses.clone()));
  thread::spawn(defibrillator(shared.clone(), sender, sockets.clone(), statuses.clone()));
  thread::spawn(worker(shared.clone(), gig_rx));
  thread::spawn(commander(shared.clone(), command_rx, command_sender, sockets.clone()));

  Ok(command_tx)
}