pub mod flight_computer;
pub mod communicators;
pub mod discovery;
use fs_protobuf_rust::compiled::mcfs::device::DeviceType;
use std::{thread, net::{SocketAddr, IpAddr, Ipv4Addr}};
use discovery::get_ips;
use fc::{communicators::{board_communicator::{self, BoardCommunicator}, Communicator, server_communicator::{ControlServerCommunicator, self}}};
use std::sync::mpsc;
use flight_computer::state;

fn main() {
    // let mut discover = DeviceDiscovery::new();
    // // https://stackoverflow.com/questions/26732763/udpsocket-send-to-fails-with-invalid-argument 
    // let board_comm_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 7777);
    // let sock_comm_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 5025);
    // let mut board_comm = BoardCommunicator::new(board_comm_addr);
    // let mut server_comm = ControlServerCommunicator::new(sock_comm_addr);

    // // discovery to board comm
    // let (tx1, rx1) = mpsc::channel();

    // // discovery to server comm
    // let (tx2, rx2) = mpsc::channel();

    // // server comm to board comm 
    // let (tx3, rx3) = mpsc::channel();

    // // board comm to server comm
    // //let (tx4, rx4) = mpsc::channel();

    // let discovery_loop = thread::spawn(move || {
    //     discovery::init_mcast(&mut discover);

    //     loop {
    //         discovery::recv_mcast(&mut discover);

    //         tx1.send((discover.mappings).clone()).unwrap();

    //         // directly send server address
    //         for (_, (dev_type, addr)) in (discover.mappings).clone() {
    //             if dev_type == DeviceType::SERVER {
    //                 tx2.send(addr).unwrap();
    //             }
    //         }
    //     }
    // });

    // let server_comm_loop = thread::spawn(move || {
    //     let server_addr = rx2.recv().unwrap();
    //     println!("server address over channel: {:?}", server_addr);

    //     loop {
    //         let server_recv = server_communicator::begin(&mut server_comm, server_addr);
    //         tx3.send(server_recv).unwrap();
    //     }
    // });

    // let board_comm_loop = thread::spawn(move || {
    //     let hashmap = rx1.recv().unwrap();
    //     board_comm.update_mappings(hashmap);

    //     loop {
    //         let server_forwarded = rx3.recv().unwrap();
    //         board_communicator::begin(&mut board_comm, server_forwarded);
    //     }
    // });

    // discovery_loop.join();
    // server_comm_loop.join();
    // board_comm_loop.join();
    // let hostnames= ["flight-computer-01.local", "server-01.local", "sam-01.local"];
    // let ips = get_ips(&hostnames);
    // println!("ips: {:?}", ips);
    // assert!(ips.contains(&"127.0.0.1".parse().unwrap()));
    //let mut flight_computer = flight_computer::FlightComputer::new();
    let mut fc_state = state::State::Init;
    let mut data = state::Data::new();
    loop {
        fc_state = fc_state.next(&mut data);
        // thread::sleep(std::time::Duration::from_secs(3));
    }


}




