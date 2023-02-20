use std::net::{SocketAddr, UdpSocket};
use crate::command_parser;

// Board Communicator runs on the FS Flight Computer
// Relays messages to and from the Control Server Communicator, the SAM modules, and the BMS
// Uses the User Datagram Protocol (UDP)
pub struct BoardCommunicator {
    addr: SocketAddr,
    socket: Option<UdpSocket>,
    deployed: bool,
}

impl BoardCommunicator {
    // Constructs a new instance of ['BoardCommunicator']
    pub fn new(addr: SocketAddr) -> BoardCommunicator {
        BoardCommunicator {
            addr,
            socket: None, 
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
            println!("{:?} bytes sent", sent_bytes);
            return sent_bytes;
        } 
        
        panic!("The socket hasn't been initialized yet");
    }

    // Reads in data over UDP
    pub fn listen(&self, buf: &mut Vec<u8>) -> (usize, SocketAddr) {
        if let Some(ref socket) = self.socket {
            let (num_bytes, src_addr) = socket.recv_from(buf).expect("Failed to receive data");
            println!("{:?} bytes received from {:?}", num_bytes, src_addr);

            // route data to destination denoted in message header
            if let Some(routing_addr) = command_parser::parse(&buf) {
                self.send(buf, routing_addr);
            }

            return (num_bytes, src_addr);
        } 

        panic!("The socket hasn't been initialized yet");
    }
}