use std::{collections::HashMap, net::{IpAddr, SocketAddr, UdpSocket}, io::Read};
use quick_protobuf::{deserialize_from_slice, Error};

use crate::discovery::get_ips;
use std::net::TcpStream;

use fs_protobuf_rust::compiled::mcfs::core;

const SERVER_ADDR: &str = "Jeffs-MacBook-Pro.local";
const HOSTNAMES: [&str; 2] = [SERVER_ADDR, "sam-01.local"];

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
                        data.server.as_ref().unwrap().set_nonblocking(false).expect("set_nonblocking call failed");
                        return State::HandleCommands
                    },
                    Err(_e) => {
                        return State::DeviceDiscovery
                    }
                }
            }

            State::HandleCommands => {
                let mut buf = [0; 2000];
                data.server.as_mut().unwrap().read(&mut buf).expect("No data received");
                let deserialized: Result<core::Message, Error> = deserialize_from_slice(&buf);
                println!("{:?}", deserialized);

                if let Some(ip) = data.ip_addresses.get("sam-01.local") {
                    match ip {
                        Some(ipv4_addr) => {
                            let socket_addr = SocketAddr::new(*ipv4_addr, 8378);
                            let socket = UdpSocket::bind("0.0.0.0:9572").expect("couldn't bind to address");
                            socket.connect(socket_addr).expect("connect function failed");
                            socket.send(&buf).expect("couldn't send message");
                            return State::HandleCommands
                        },
                        None => {
                            return State::DeviceDiscovery
                        }
                    }
                } else {
                    return State::DeviceDiscovery
                }
            }
        }
    }
    
}