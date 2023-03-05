use fs_protobuf_rust::compiled::mcfs::command;
use fs_protobuf_rust::compiled::mcfs::core;
use quick_protobuf::deserialize_from_slice;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use lazy_static::lazy_static;

// FS FC sends a broadcast message to all connected devices/modules to dynamically store 
// destination addresses for message routing and identification purposes

// temporary hard-coding of device addresses 
// hashmap of board id to ip address mappings 

lazy_static! {
    static ref HASHMAP: HashMap<u32, SocketAddr> = {
        let mut mappings = HashMap::new();
        // 0 = FC (route to self)
        mappings.insert(0, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));
        mappings.insert(1, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081));
        mappings.insert(2, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8082));
        mappings.insert(3, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8083));
        mappings.insert(4, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8084));
        mappings.insert(5, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8085));
        mappings
    };
}

// Demultiplexes protobuf messages via the NodeIdentifier and determines which board to send data to
pub fn parse(message: &Vec<u8>) -> Option<&SocketAddr> {
    // deserialize message
    let data: core::Message = deserialize_from_slice(&message).expect("Cannot deserialize message");

    match data.content {
        core::mod_Message::OneOfcontent::command(c) => 
            match c.command {
                command::mod_Command::OneOfcommand::data_directive(directive_cmd) => 
                    if let Some(node) = directive_cmd.node {
                        if let Some(address) = HASHMAP.get(&node.board_id) {
                            Some(address)
                        } else {
                            panic!("Couldn't get address mapping")
                        }
                    } else {
                        panic!("Couldn't access node's board id")
                    },
                    
                command::mod_Command::OneOfcommand::click_valve(valve_cmd) => 
                if let Some(valve) = valve_cmd.valve {
                    if let Some(address) = HASHMAP.get(&valve.board_id) {
                        Some(address)
                    } else {
                        panic!("Couldn't get address mapping")
                    }
                } else {
                    panic!("Couldn't access node's board id")
                },
                
                command::mod_Command::OneOfcommand::set_led(led_cmd) => 
                if let Some(led) = led_cmd.led {
                    if let Some(address) = HASHMAP.get(&led.board_id) {
                        Some(address)
                    } else {
                        panic!("Couldn't get address mapping")
                    }
                } else {
                    panic!("Couldn't access node's board id")
                },

                command::mod_Command::OneOfcommand::device_discovery(..) => None,

                command::mod_Command::OneOfcommand::None => None,
            }

        core::mod_Message::OneOfcontent::data(..) => None,

        core::mod_Message::OneOfcontent::status(..) => None,

        _ => None,

    }
}
