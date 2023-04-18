pub mod flight_computer;
pub mod communicators;
pub mod discovery;
use std::{thread, net::{SocketAddr, IpAddr, Ipv4Addr}};
use discovery::DeviceDiscovery;
use fc::{communicators::{board_communicator::{self, BoardCommunicator}, Communicator}};
use std::sync::mpsc;

fn main() {
    let mut discover = DeviceDiscovery::new();
    let sock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let mut board_comm = BoardCommunicator::new(sock_addr);

    let (tx1, rx1) = mpsc::channel();

    let discovery_loop = thread::spawn(move || {
        discovery::init_mcast(&mut discover);
        loop {
            discovery::recv_mcast(&mut discover);
            println!("{:?}", discover.mappings.get(&1));

            tx1.send((discover.mappings).clone()).unwrap();
        }
    });

    let board_comm_loop = thread::spawn(move || {
        println!("before update mappings");
        let hashmap = rx1.recv().unwrap();

        board_comm.update_mappings(hashmap);
        loop {
            board_communicator::begin(&mut board_comm);
        }
    });

    discovery_loop.join();
    board_comm_loop.join();
}




