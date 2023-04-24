use std::net::{SocketAddr, UdpSocket, TcpStream};
use std::io::prelude::*;

// The Control Server Communicator runs on the FS Flight Computer
// Uses TCP to communicate with the Control Server and UDP to the Board Communicator 
pub struct ControlServerCommunicator {
    addr: SocketAddr,
    socket: Option<UdpSocket>,
    server: Option<TcpStream>,
    deployed: bool,
}

pub fn begin(server_comm: &mut ControlServerCommunicator, server_addr: SocketAddr) -> Vec<u8> {
    server_comm.server_connect(&server_addr);

    // listen for messages from server 
    let server_recv = server_comm.listen_server();
    
    return server_recv;
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

    // Sends data to the Control Server over TCP
    pub fn send_server(&mut self, message: &Vec<u8>) -> usize {
        if let Some(ref mut server) = self.server {
            let sent_bytes = server.write(message).expect("Failed to send message");
            println!("{:?} bytes sent from {:?}", sent_bytes, self.addr);
            return sent_bytes;
        }

        panic!("The stream hasn't been initialized yet");
    }

    // Reads in data from the control server over TCP
    pub fn listen_server(&mut self) -> Vec<u8> {
        let mut buf = vec![0; 5000];

        if let Some(ref mut stream) = self.server {
            let num_bytes = stream.read(&mut buf).expect("Failed to receive data from control server");
            println!("{:?} bytes received", num_bytes);

            return buf;
        } 
        panic!("The server stream hasn't been initialized yet");
    }   

    // Reads in data over UDP 
    // pub fn listen_board(&mut self, buf: &mut Vec<u8>) -> (usize, SocketAddr) {
    //     if let Some(ref socket) = self.socket {
    //         let (num_bytes, src_addr) = socket.recv_from(buf).expect("Failed to receive data");
    //         println!("{:?} bytes received from {:?}", num_bytes, src_addr);

    //         let (_board_id, _, routing_addr) = self.parse(&buf);

    //         if let Some(addr) = routing_addr {
    //             if addr.ip() == IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)) {
    //                 // send to FC (self)
    //                 self.send_udp(buf, addr);
    //             } else {
    //                 self.send_server(buf);
    //             }
    //         }

    //         return (num_bytes, src_addr);
    //     } 
    //     panic!("The socket hasn't been initialized yet");
    // }
}