use std::net::{SocketAddr, UdpSocket, Ipv4Addr, IpAddr};
use std::collections::HashMap;
use crate::communicators::Communicator;
use fs_protobuf_rust::compiled::google::protobuf::Timestamp;
use fs_protobuf_rust::compiled::mcfs::{core, command, device};
use fs_protobuf_rust::compiled::mcfs::device::DeviceType;
use quick_protobuf::serialize_into_vec;

// Board Communicator runs on the FS Flight Computer
// Relays messages to and from the Control Server Communicator, the SAM modules, and the BMS
// Uses the User Datagram Protocol (UDP)
pub struct BoardCommunicator {
    addr: SocketAddr,
    socket: Option<UdpSocket>,
    mappings: HashMap<u32, (DeviceType, SocketAddr)>,
    deployed: bool,
}

pub fn begin(board_comm: &mut BoardCommunicator) {
    board_comm.send_bind();
    
    // PROTOBUF MESSAGE STARTS HERE 
    let command = command::Command {
        command: command::mod_Command::OneOfcommand::click_valve(
            command::ClickValve { 
                valve: (Some(device::NodeIdentifier {board_id: 1, channel: device::Channel::VALVE, node_id: 0})), 
                state: (device::ValveState::VALVE_OPEN)
    })};

    let command_message = core::Message {
        timestamp: Some(Timestamp {seconds: 1, nanos: 100}),
        board_id: 5,
        content: core::mod_Message::OneOfcontent::command(command)
    };

    let data_serialized = serialize_into_vec(&command_message).expect("Cannot serialize `data`");
     // PROTOBUF MESSAGE ENDS HERE 

    let destination = board_comm.get_mappings(&1);

    // ADDRESS BELOW FOR COMMAND LOOP SOCKET ON SAM 
    //let sam_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(169, 254, 42, 143)), 8378);

    loop {
        if let Some((_, address)) = destination {
            println!("address: {:?}", address.to_string());
            let sent_bytes = board_comm.send(&data_serialized, address);
            println!("bytes sent: {:?}", sent_bytes);
        }
    }
}

impl Communicator for BoardCommunicator {
    fn get_mappings(&self, board_id: &u32) -> Option<(DeviceType, &SocketAddr)> {
        if let Some((dev_type, address)) = self.mappings.get(board_id) {
            Some((*dev_type, address))
        } else {
            panic!("Couldn't access mapping")
        }
    }

    fn update_mappings(&mut self, new_hashmap: HashMap<u32, (DeviceType, SocketAddr)>) -> HashMap<u32, (DeviceType, SocketAddr)> {
        println!("inside update mappings");

        for (key, value) in new_hashmap.iter() {
            self.mappings.insert(*key, *value);
        }

        self.mappings.clone()
    }
}

impl BoardCommunicator {
    // Constructs a new instance of ['BoardCommunicator']
    pub fn new(addr: SocketAddr) -> BoardCommunicator {
        BoardCommunicator {
            addr,
            socket: None, 
            mappings: HashMap::new(),
            deployed: false,
        }
    }

    // Attaches a UDP socket to the provided IP address and port 
    pub fn send_bind(&mut self) {
        if let Ok(socket) = UdpSocket::bind(self.addr) {
            self.socket = Some(socket);
            self.deployed = true;
        } else {
            panic!("Could not attach socket to address and port");
        }
    }

    pub fn send(&self, message: &Vec<u8>, dst: &SocketAddr) -> usize {
        if let Some(ref socket) = self.socket {
            println!("message: {:?}, dst: {:?}", message, dst.to_string().as_str());
            let sent_bytes = socket.send_to(message, dst.to_string()).expect("failed to send message");
            println!("{:?} bytes sent from {:?}", sent_bytes, self.addr);
            return sent_bytes;
        } 
        
        panic!("The socket hasn't been initialized yet");
    }

    // Reads in data over UDP
    pub fn listen(&mut self, buf: &mut Vec<u8>) -> (usize, SocketAddr) {
        if let Some(ref socket) = self.socket {
            let (num_bytes, src_addr) = socket.recv_from(buf).expect("Failed to receive data");
            println!("{:?} bytes received from {:?}", num_bytes, src_addr);

            let (_, _, routing_addr) = self.parse(&buf);

            if let Some(addr) = routing_addr {
                self.send(buf, addr);
            }

            return (num_bytes, src_addr);
        } 

        panic!("The socket hasn't been initialized yet");
    }
}

