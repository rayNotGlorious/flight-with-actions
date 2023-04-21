pub mod flight_computer;
pub mod communicators;
pub mod discovery;
use std::{thread, net::{SocketAddr, IpAddr, Ipv4Addr}};
use discovery::DeviceDiscovery;
use fc::{communicators::{board_communicator::{self, BoardCommunicator}, Communicator, server_communicator::{ControlServerCommunicator, self}}};
use std::sync::mpsc;

fn main() {
    let mut discover = DeviceDiscovery::new();
    // https://stackoverflow.com/questions/26732763/udpsocket-send-to-fails-with-invalid-argument 
    let board_comm_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 7777);
    let sock_comm_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 5025);
    let mut board_comm = BoardCommunicator::new(board_comm_addr);
    let mut server_comm = ControlServerCommunicator::new(sock_comm_addr);

    let (tx1, rx1) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    let discovery_loop = thread::spawn(move || {
        discovery::init_mcast(&mut discover);

        loop {
            discovery::recv_mcast(&mut discover);
            println!("{:?}", discover.mappings.get(&1));

            tx1.send((discover.mappings).clone()).unwrap();
            tx2.send((discover.mappings).clone()).unwrap();
        }
    });

    let board_comm_loop = thread::spawn(move || {
        let hashmap = rx1.recv().unwrap();
        board_comm.update_mappings(hashmap);

        loop {
            board_communicator::begin(&mut board_comm);
        }
    });

    let server_comm_loop = thread::spawn(move || {
        let hashmap = rx2.recv().unwrap();
        server_comm.update_mappings(hashmap);

        loop {
            server_communicator::begin(&mut server_comm)
        }
    });

    discovery_loop.join();
    board_comm_loop.join();
    //server_comm_loop.join();
}




