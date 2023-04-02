use std::net::{SocketAddr, UdpSocket};
use std::collections::HashMap;
use crate::communicators::Communicator;

// Board Communicator runs on the FS Flight Computer
// Relays messages to and from the Control Server Communicator, the SAM modules, and the BMS
// Uses the User Datagram Protocol (UDP)
pub struct BoardCommunicator {
    addr: SocketAddr,
    socket: Option<UdpSocket>,
    mappings: HashMap<u32, SocketAddr>,
    deployed: bool,
}

impl Communicator for BoardCommunicator {
    fn get_mappings(&self, board_id: &u32) -> Option<&SocketAddr> {
        if let Some(address) = self.mappings.get(board_id) {
            Some(address)
        } else {
            panic!("Couldn't get address mapping")
        }
    }

    fn set_mappings(&mut self, board_id: u32, ip_addr: SocketAddr) {
        self.mappings.insert(board_id, ip_addr);
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
            let sent_bytes = socket.send_to(message, &dst).expect("failed to send message");
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

            let (_, routing_addr) = self.parse(&buf);

            if let Some(addr) = routing_addr {
                self.send(buf, addr);
            }

            return (num_bytes, src_addr);
        } 

        panic!("The socket hasn't been initialized yet");
    }
}

