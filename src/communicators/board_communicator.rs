use std::net::{SocketAddr, UdpSocket, IpAddr, Ipv4Addr};
use std::process::Command;
use std::str::FromStr;
use crate::command_parser;

// Board Communicator runs on the FS Flight Computer
// Relays messages to and from the Control Server Communicator, the SAM modules, and the BMS
// Uses the User Datagram Protocol (UDP)
pub struct BoardCommunicator {
    addr: SocketAddr,
    socket: Option<UdpSocket>,
    pub mcast: Option<SocketAddr>,
    deployed: bool,
}

impl BoardCommunicator {
    // Constructs a new instance of ['BoardCommunicator']
    pub fn new(addr: SocketAddr) -> BoardCommunicator {
        BoardCommunicator {
            addr,
            socket: None, 
            mcast: None,
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

    pub fn multicast_setup(&mut self) {
        let mcast = Ipv4Addr::new(224, 0, 0, 1);
        let mut any = Ipv4Addr::new(192, 168, 6, 2);

        // gets Beaglebone interface IP 
        let network_ip = Command::new("sh")
                            .arg("-c")
                            .arg("ip addr show eth0 | grep inet | awk '{print $2}' | cut -d '/' -f1")
                            .output()
                            .expect("failed to execute process");
                    
        if network_ip.stdout.len() != 0 {
            let stdout = String::from_utf8_lossy(&network_ip.stdout);
            any = Ipv4Addr::from_str(stdout.trim()).expect("failed to parse network IP address");
        }

        let mcast_addr = SocketAddr::new(IpAddr::V4(mcast), 6000);
        self.mcast = Some(mcast_addr);

        if let Some(ref socket) = self.socket {
            socket.join_multicast_v4(&mcast, &any);
        } else {
            panic!("Failed to join multicast group");
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
    pub fn listen(&self, buf: &mut Vec<u8>) -> (usize, SocketAddr) {
        if let Some(ref socket) = self.socket {
            let (num_bytes, src_addr) = socket.recv_from(buf).expect("Failed to receive data");
            println!("{:?} bytes received from {:?}", num_bytes, src_addr);

            // route data to destination denoted in message header
            if let Some(routing_addr) = command_parser::parse(&buf) {
                println!("routing address: {:?}", routing_addr);
                self.send(buf, routing_addr);
            }

            return (num_bytes, src_addr);
        } 

        panic!("The socket hasn't been initialized yet");
    }
}