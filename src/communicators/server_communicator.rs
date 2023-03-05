use std::net::{SocketAddr, UdpSocket, TcpStream, IpAddr, Ipv4Addr};
use std::io::prelude::*;
use crate::command_parser;

// The Control Server Communicator runs on the FS Flight Computer
// Uses TCP to communicate with the Control Server and UDP to the Board Communicator 
pub struct ControlServerCommunicator {
    addr: SocketAddr,
    socket: Option<UdpSocket>,
    server: Option<TcpStream>,
    deployed: bool,
}

impl ControlServerCommunicator {
    // Constructs a new instance of ['ControlServerCommunicator']
    pub fn new(addr: SocketAddr) -> ControlServerCommunicator {
        ControlServerCommunicator {
            addr, 
            socket: None,
            server: None,
            deployed: false,
        }
    }

    // Connected to the Control Server via TCP
    pub fn server_connect(&mut self, server_addr: &SocketAddr) {
        if let Ok(server) = TcpStream::connect(server_addr) {
            self.server = Some(server);
        } else {
            panic!("Failed to connect");
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

    // Sends data to the Control Server over TCP
    pub fn send_server(&mut self, message: &Vec<u8>) -> usize {
        if let Some(ref mut server) = self.server {
            let sent_bytes = server.write(message).expect("Failed to send message");
            println!("{:?} bytes sent from {:?}", sent_bytes, self.addr);
            return sent_bytes;
        }

        panic!("The stream hasn't been initialized yet");
    }

    // Sends data to the Board Communicator over UDP
    pub fn send_udp(&self, message: &Vec<u8>, dst: &SocketAddr) -> usize {
        if let Some(ref socket) = self.socket {
            let sent_bytes = socket.send_to(message, &dst).expect("failed to send message");
            println!("{:?} bytes sent from {:?}", sent_bytes, self.addr);
            return sent_bytes;
        } 
        
        panic!("The socket hasn't been initialized yet");
    }

    // Reads in data from the control server over TCP 
    pub fn listen_server(&mut self, buf: &mut Vec<u8>) -> usize {
        if let Some(ref mut stream) = self.server {
            let num_bytes = stream.read(buf).expect("Failed to receive data from control server");
            println!("{:?} bytes received", num_bytes);

            // route data to destination denoted in message header
            if let Some(routing_addr) = command_parser::parse(&buf) {
                self.send_udp(buf, routing_addr);
            }

            return num_bytes;
        } 
        panic!("The server stream hasn't been initialized yet");
    }   

    // Reads in data over UDP and stores it in a circular buffer
    pub fn listen_board(&mut self, buf: &mut Vec<u8>) -> (usize, SocketAddr) {
        if let Some(ref socket) = self.socket {
            let (num_bytes, src_addr) = socket.recv_from(buf).expect("Failed to receive data");
            println!("{:?} bytes received from {:?}", num_bytes, src_addr);
            
            // route data to destination denoted in message header
            if let Some(routing_addr) = command_parser::parse(&buf) {
                if routing_addr.ip() == IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)) {
                    // send to FC (self)
                    self.send_udp(buf, routing_addr);
                } else {
                    self.send_server(buf);
                }
            }

            return (num_bytes, src_addr);
        } 
        panic!("The socket hasn't been initialized yet");
    }
}