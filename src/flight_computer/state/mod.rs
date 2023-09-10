use std::{collections::HashMap, net::{IpAddr, SocketAddr}, io::Read};
use quick_protobuf::{deserialize_from_slice, Error};

use crate::discovery::get_ips;
use std::net::TcpStream;

use fs_protobuf_rust::compiled::mcfs::core;

const SERVER_ADDR: &str = "localhost";
const HOSTNAMES: [&str; 1] = [SERVER_ADDR];

pub struct Data {
    ip_addresses: HashMap<String, Option<IpAddr>>,
    server: Option<TcpStream>,
    state_num: u32,
}

impl Data {
    pub fn new() -> Data {
        Data {
            ip_addresses: HashMap::new(),
            server: None,
            state_num: 0,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum State {
    Init,
    // SoftwareSystemCheck,
    // ReadLocalConfigs,
    DeviceDiscovery,
    ConnectToServer,
    HandleCommands,
}

impl State {
    pub fn next(self, data: &mut Data) -> State {
        println!("{:?} {}", self, data.state_num);
        data.state_num += 1;
        match self {
            State::Init => {
                State::DeviceDiscovery
            }

            State::DeviceDiscovery => {
                data.ip_addresses = get_ips(&HOSTNAMES);
                if let Some(ip) = data.ip_addresses.get(SERVER_ADDR) {
                    match ip {
                        Some(_ipv4_addr) => {
                            State::ConnectToServer
                        },
                        None => {
                            State::DeviceDiscovery
                        }
                    }
                } else {
                    State::DeviceDiscovery
                }
            }

            State::ConnectToServer => {
                let server_addr = data.ip_addresses.get(SERVER_ADDR).unwrap().unwrap();
                let socket_addr = SocketAddr::new(server_addr, 5025);
                match TcpStream::connect(socket_addr) {
                    Ok(stream) => {
                        data.server = Some(stream);
                        return State::HandleCommands
                    },
                    Err(_e) => {
                        return State::DeviceDiscovery
                    }
                }
            }

            State::HandleCommands => {
                let mut buf = [0; 65536];
                data.server.as_mut().unwrap().read(&mut buf).expect("No data received");
                let deserialized: Result<core::Message, Error> = deserialize_from_slice(&buf);
                println!("{:?}", deserialized);
                State::HandleCommands
            }
        }
    }
    
}