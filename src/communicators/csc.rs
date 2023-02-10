use std::net::UdpSocket;
use std::net::SocketAddr;
//use circular::Buffer;
use fs_protobuf_rust::compiled::mcfs::command;
use fs_protobuf_rust::compiled::mcfs::core;
use quick_protobuf::deserialize_from_slice;
use crate::destinations::address_mapping;


// The Control Server Communicator (CSC) runs on the FS Flight Computer
// Relays messages to and from the FS Control Server, the SAM modules, and the BMS 
pub struct ControlServerCommunicator {
    socket: Option<UdpSocket>,
    port: u16,
    deployed: bool,
}

impl ControlServerCommunicator {
    // Constructs a new instance of ['ControlServerCommunicator']
    pub fn new(port: u16) -> ControlServerCommunicator {
        ControlServerCommunicator {
            socket: None,
            port, 
            deployed: false,
        }
    }

    // Attaches a UDP socket to the provided IP address and port 
    // Socket is available to start receiving messages 
    pub fn send_bind(&mut self) {
        if let Ok(socket) = UdpSocket::bind(format!("0.0.0.0:{}", self.port)) {
            self.socket = Some(socket);
            self.deployed = true;
        } else {
            panic!("Could not attach socket to address and port");
        }
    }

    pub fn send(&self, message: &Vec<u8>, dst: &SocketAddr) -> usize {
        //let (dst_addr, dst_port) = Self::check_dst(dst)?;
        if let Some(ref socket) = self.socket {
            let sent_bytes = socket.send_to(message, &dst).expect("failed to send message");
            println!("{:?} bytes sent", sent_bytes);
            return sent_bytes;
        } 
        
        panic!("The socket hasn't been initialized yet");
    }

    // Checks the destination format matches 'address:port'
    // fn check_dst(dst: &str) -> Result<(String, u16), Box<dyn std::error::Error>> {
    //     let mut sections = dst.split(":");
    //     let address = sections.next().ok_or("invalid destination format")?;
    //     let port = sections.next().ok_or("invalid destination format")?;
    //     let port = port.parse::<u16>().map_err(|_| "invalid port number")?;
    //     Ok((address.to_string(), port))
    // }

    // Reads in data over UDP and stores it in a circular buffer
    pub fn listen(&self, buf: &mut Vec<u8>) -> (usize, SocketAddr) {
        if let Some(ref socket) = self.socket {
            let (num_bytes, src_addr) = socket.recv_from(buf).expect("no data received");
            println!("{:?} bytes received from {:?}", num_bytes, src_addr);
            self.route(buf);
            return (num_bytes, src_addr);
        } 

        panic!("The socket hasn't been initialized yet");
    }

    // Demultiplexes protobuf messages via the NodeIdentifier and determines which board to send data to
    pub fn route(&self, message: &Vec<u8>) -> bool {
        // deserialize message
        let data: core::Message = deserialize_from_slice(&message).expect("Cannot deserialize message");

        match data.content {
            core::mod_Message::OneOfcontent::command(c) => 
                match c.command {
                    command::mod_Command::OneOfcommand::data_directive(directive_cmd) => 
                        if let Some(node) = directive_cmd.node {
                            self.send(&message, &address_mapping(node.board_id));
                            true
                        } else {
                            panic!("Couldn't access node's board id")
                        },
                        
                    command::mod_Command::OneOfcommand::click_valve(valve_cmd) => 
                    if let Some(valve) = valve_cmd.valve {
                        self.send(&message, &address_mapping(valve.board_id));
                        true
                    } else {
                        panic!("Couldn't access node's board id")
                    },
                    
                    command::mod_Command::OneOfcommand::set_led(led_cmd) => 
                    if let Some(led) = led_cmd.led {
                        self.send(&message, &address_mapping(led.board_id));
                        true
                    } else {
                        panic!("Couldn't access node's board id")
                    },

                    command::mod_Command::OneOfcommand::None => false,
                }

            core::mod_Message::OneOfcontent::data(..) => true,

            core::mod_Message::OneOfcontent::status(..) => true,

            _ => false,

        }
    }
}