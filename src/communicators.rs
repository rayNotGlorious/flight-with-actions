pub mod server_communicator;
pub mod board_communicator;
use std::net::{SocketAddr, Ipv4Addr, UdpSocket};
use fs_protobuf_rust::compiled::mcfs::{command};
use fs_protobuf_rust::compiled::mcfs::core;
use fs_protobuf_rust::compiled::mcfs::device;
use fs_protobuf_rust::compiled::mcfs::status;
use quick_protobuf::{serialize_into_vec, deserialize_from_slice};

pub trait Communicator {
    fn get_mappings(&self, board_id: &u32) -> Option<&SocketAddr>;
    fn set_mappings(&mut self, board_id: u32, ip_addr: SocketAddr);

    fn device_discovery(&mut self) {
        let mcast_group: Ipv4Addr = "224.0.0.3".parse().unwrap();
        let port: u16 = 6000;
        let any = "0.0.0.0".parse().unwrap();

        let socket = UdpSocket::bind((any, port)).expect("Could not bind client socket");
        socket.set_multicast_loop_v4(false).expect("set_multicast_loop_v4 call failed");
        socket
            .join_multicast_v4(&mcast_group, &any)
            .expect("Could not join multicast group");

        let response = core::Message {
            timestamp: None,
            board_id: 1,
            content: core::mod_Message::OneOfcontent::status(status::Status {
                status_message: std::borrow::Cow::Borrowed(""),
                status: status::mod_Status::OneOfstatus::device_info(status::DeviceInfo {
                    board_id: 1, 
                    device_type: device::DeviceType::FLIGHT_COMPUTER 
                })
            }),
        };
    
        let response_serialized = serialize_into_vec(&response).expect("Could not serialize discovery response");

        let mut buffer = [0u8; 1600];

        loop {
            let result = socket.recv_from(&mut buffer);
            match result {
                Ok((_size, src)) => {
                    // TODO: log discovery message
                    println!("Received discovery message from {}", src);
                    let _result = socket.send_to(&response_serialized, &(mcast_group, port));
                }
                Err(_e) => {
                    // TODO: log error
                }
            }
        }
    }
    
    // Demultiplexes protobuf messages via the NodeIdentifier and determines which board to send data to
    fn parse(&self, message: &Vec<u8>) -> (Option<u32>, Option<&SocketAddr>) {
        // deserialize message
        let data: core::Message = deserialize_from_slice(&message).expect("Cannot deserialize message");

        match data.content {
            core::mod_Message::OneOfcontent::command(c) => 
                match c.command {
                    command::mod_Command::OneOfcommand::data_directive(directive_cmd) => 
                        if let Some(node) = directive_cmd.node {
                            if let Some(address) = self.get_mappings(&node.board_id) {
                                (Some(node.board_id), Some(address))
                            } else {
                                panic!("Couldn't get address mapping")
                            }
                        } else {
                            panic!("Couldn't access node's board id")
                        },
                        
                    command::mod_Command::OneOfcommand::click_valve(valve_cmd) => 
                    if let Some(valve) = valve_cmd.valve {
                        if let Some(address) = self.get_mappings(&valve.board_id) {
                            (Some(valve.board_id), Some(address))
                        } else {
                            panic!("Couldn't get address mapping")
                        }
                    } else {
                        panic!("Couldn't access node's board id")
                    },
                    
                    command::mod_Command::OneOfcommand::set_led(led_cmd) => 
                    if let Some(led) = led_cmd.led {
                        if let Some(address) = self.get_mappings(&led.board_id) {
                            (Some(led.board_id), Some(address))
                        } else {
                            panic!("Couldn't get address mapping")
                        }
                    } else {
                        panic!("Couldn't access node's board id")
                    },

                    command::mod_Command::OneOfcommand::device_discovery(..) => (None, None),

                    command::mod_Command::OneOfcommand::None => (None, None),
                }

            core::mod_Message::OneOfcontent::data(..) => (None, None),

            core::mod_Message::OneOfcontent::status(s) => 
                match s.status {
                    status::mod_Status::OneOfstatus::device_info(info) =>
                        (Some(info.board_id), None),
                    
                    status::mod_Status::OneOfstatus::device_status(..) => (None, None),

                    status::mod_Status::OneOfstatus::channel_status(..) => (None, None),

                    status::mod_Status::OneOfstatus::node_status(..) => (None, None),

                    status::mod_Status::OneOfstatus::None => (None, None),

                }

            _ => (None, None),

        }
    }
}