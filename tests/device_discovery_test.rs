use std::{thread, net::{SocketAddr, IpAddr, Ipv4Addr}};
use fc::{discovery::{self, DeviceDiscovery}, communicators::{board_communicator::{self, BoardCommunicator}, Communicator}};
use std::sync::mpsc;

#[test]
fn device_discovery_test() {
    let device_discovery = DeviceDiscovery::new();
    let sock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let mut board_comm = BoardCommunicator::new(sock_addr);

    let (tx1, rx1) = mpsc::channel();

    let _discovery_loop = thread::spawn(move || {
        discovery::begin();
        loop {
            tx1.send((device_discovery.mappings).clone()).unwrap();
        }
    });

    let _board_comm_loop = thread::spawn(move || {
        board_communicator::begin(&mut board_comm);
        loop {
            let hashmap = rx1.recv().unwrap();
            board_comm.update_mappings(&hashmap);
        }
    });

    //assert_eq!(true, device_discovery.mappings.clone().contains_key(&1));
    //assert_eq!(device_discovery.mappings.clone().get(&1), board_comm.get_mappings(&1));
}


