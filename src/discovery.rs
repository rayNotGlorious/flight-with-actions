use std::net::{SocketAddr, UdpSocket, Ipv4Addr};
use std::collections::HashMap;
use fs_protobuf_rust::compiled::mcfs::{core};
use fs_protobuf_rust::compiled::mcfs::device::{self, DeviceType};
use fs_protobuf_rust::compiled::mcfs::status;
use quick_protobuf::{serialize_into_vec, deserialize_from_slice};

pub struct DeviceDiscovery {
    pub mappings: HashMap<u32, (DeviceType, SocketAddr)>,
}

impl DeviceDiscovery {
    pub fn new() -> DeviceDiscovery {
        DeviceDiscovery {
            mappings: HashMap::new(),
        }
    }
}

fn parse(message: &Vec<u8>) -> (Option<u32>, Option<DeviceType>, Option<&SocketAddr>)  {
    // deserialize message
    let data: core::Message = deserialize_from_slice(&message).expect("Cannot deserialize message");

    match data.content {
        core::mod_Message::OneOfcontent::status(s) => 
            match s.status {
                status::mod_Status::OneOfstatus::device_info(info) =>
                    (Some(info.board_id), Some(info.device_type), None),
                _ => (None, None, None)
            }
        _ => (None, None, None),
    }
}

pub fn begin() {
    let mut device_discovery = DeviceDiscovery::new();

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

    let mut buffer = vec![0u8; 1600];

    loop {
        let result = socket.recv_from(&mut buffer);
        match result {
            Ok((_size, src)) => {
                // TODO: log discovery message
                let (board_id, device_type, _) = parse(&buffer);

                if let Some(id) = board_id {
                    if let Some(dev_type) = device_type {
                        device_discovery.mappings.insert(id, (dev_type, src));
                    }
                }

                println!("Received discovery message from {}", src);
                let _result = socket.send_to(&response_serialized, &(mcast_group, port));
            }
            Err(_e) => {
                // TODO: log error
            }
        }
    }
}