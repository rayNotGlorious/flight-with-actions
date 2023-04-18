pub mod server_communicator;
pub mod board_communicator;
use std::collections::HashMap;
use std::net::{SocketAddr};
use fs_protobuf_rust::compiled::mcfs::{command};
use fs_protobuf_rust::compiled::mcfs::core;
use fs_protobuf_rust::compiled::mcfs::status;
use quick_protobuf::{deserialize_from_slice};
use fs_protobuf_rust::compiled::mcfs::device::DeviceType;

pub trait Communicator {
    fn get_mappings(&self, board_id: &u32) -> Option<(DeviceType, &SocketAddr)>;
    fn update_mappings(&mut self, new_hashmap: HashMap<u32, (DeviceType, SocketAddr)>) -> HashMap<u32, (DeviceType, SocketAddr)>;
    
    // Demultiplexes protobuf messages via the NodeIdentifier and determines which board to send data to
    fn parse(&self, message: &Vec<u8>) -> (Option<u32>, Option<DeviceType>, Option<&SocketAddr>) {
        // deserialize message
        let data: core::Message = deserialize_from_slice(&message).expect("Cannot deserialize message");

        match data.content {
            core::mod_Message::OneOfcontent::command(c) => 
                match c.command {
                    command::mod_Command::OneOfcommand::data_directive(directive_cmd) => 
                        if let Some(node) = directive_cmd.node {
                            if let Some((device_type, address)) = self.get_mappings(&node.board_id) {
                                (Some(node.board_id), Some(device_type), Some(address))
                            } else {
                                panic!("Couldn't get address mapping")
                            }
                        } else {
                            panic!("Couldn't access node's board id")
                        },
                        
                    command::mod_Command::OneOfcommand::click_valve(valve_cmd) => 
                    if let Some(valve) = valve_cmd.valve {
                        if let Some((device_type, address)) = self.get_mappings(&valve.board_id) {
                            (Some(valve.board_id), Some(device_type), Some(address))
                        } else {
                            panic!("Couldn't get address mapping")
                        }
                    } else {
                        panic!("Couldn't access node's board id")
                    },
                    
                    command::mod_Command::OneOfcommand::set_led(led_cmd) => 
                    if let Some(led) = led_cmd.led {
                        if let Some((device_type, address)) = self.get_mappings(&led.board_id) {
                            (Some(led.board_id), Some(device_type), Some(address))
                        } else {
                            panic!("Couldn't get address mapping")
                        }
                    } else {
                        panic!("Couldn't access node's board id")
                    },

                    _ => (None, None, None),
                }

            core::mod_Message::OneOfcontent::status(s) => 
                match s.status {
                    status::mod_Status::OneOfstatus::device_info(info) =>
                        (Some(info.board_id), Some(info.device_type), None),
                
                    _ => (None, None, None),

                }

            _ => (None, None, None),

        }
    }
}