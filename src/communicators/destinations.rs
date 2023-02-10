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
        mappings.insert(1, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));
        mappings.insert(2, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081));
        mappings.insert(3, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8082));
        mappings.insert(4, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8083));
        mappings.insert(5, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8084));
        mappings
    };
}

pub fn address_mapping(board_id: u32) -> SocketAddr {
    if let Some(address) = HASHMAP.get(&board_id) {
        *address
    } else {
        panic!("Could not get address mapping for this board");
    }
}
