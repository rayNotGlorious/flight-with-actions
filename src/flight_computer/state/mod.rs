use std::{collections::HashMap, net::{IpAddr, SocketAddr, UdpSocket}, io::{Read, self}};
use quick_protobuf::{deserialize_from_slice, Error};

use crate::discovery::get_ips;
use std::net::TcpStream;

use fs_protobuf_rust::compiled::mcfs::{core, command};

const SERVER_ADDR: &str = "patrick-XPS-15-9500.local";
const HOSTNAMES: [&str; 2] = [SERVER_ADDR, "sam-01.local"];

pub struct Data {
    ip_addresses: HashMap<String, Option<IpAddr>>,
    board_ids: HashMap<String, u32>,
    server: Option<TcpStream>,
    state_num: u32,
    pub data_socket: UdpSocket,
}

impl Data {
    pub fn new() -> Data {
        let mut board_ids = HashMap::new();
        board_ids.insert("sam-01.local".to_string(), 1);
        board_ids.insert("sam-02.local".to_string(), 2);
        board_ids.insert("sam-03.local".to_string(), 3);
        board_ids.insert("sam-04.local".to_string(), 4);
        
        Data {
            ip_addresses: HashMap::new(),
            board_ids: board_ids,
            server: None,
            state_num: 0,
            data_socket: UdpSocket::bind("0.0.0.0:4573").expect("couldn't bind to address"),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum State {
    Init,
    DeviceDiscovery,
    ConnectToServer,
    HandleCommands,
}

impl State {
    pub fn next(self, data: &mut Data) -> State {

        if data.state_num % 100000 == 0 {
            println!("{:?} {}", self, data.state_num);
        }
        data.state_num += 1;

        match self {
            State::Init => init(data),
            State::DeviceDiscovery => device_discovery(data),
            State::ConnectToServer => connect_to_server(data),
            State::HandleCommands => handle_commands(data),
        }
    }
}

fn init(_data: &mut Data) -> State {
    State::DeviceDiscovery
}

fn device_discovery(data: &mut Data) -> State {
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

fn connect_to_server(data: &mut Data) -> State {
    let server_addr = data.ip_addresses.get(SERVER_ADDR).unwrap().unwrap();
    let socket_addr = SocketAddr::new(server_addr, 5025);
    match TcpStream::connect(socket_addr) {
        Ok(stream) => {
            data.server = Some(stream);
            data.server.as_ref().unwrap().set_nonblocking(true).expect("set_nonblocking call failed");
            return State::HandleCommands
        },
        Err(_e) => {
            return State::DeviceDiscovery
        }
    }
}

fn handle_commands(data: &mut Data) -> State {

    // receive command from server
    let mut buf = vec![];

    match data.server.as_mut().unwrap().read_to_end(&mut buf) {
        Ok(_) => {

            if buf.len() <= 0 {
                return State::DeviceDiscovery;
            } 

            let deserialized: Result<core::Message, Error> = deserialize_from_slice(&buf);

            let board_id = match deserialized {
                Ok(message) => match message.content {
                    core::mod_Message::OneOfcontent::command(command) => {
                        match command.command {
                            match get_board_id_from_command(command) {
                                Some(board_id) => {
                                    board_id
                                },
                                None => {
                                    return State::DeviceDiscovery
                                }
                            }
                        }
                    },
                    _ => {
                        return State::DeviceDiscovery
                    }
                },
                _ => {
                    return State::DeviceDiscovery
                }
            };
        

            // forward to SAM
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
        },
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
            // no new data
            return State::HandleCommands
        }
        Err(_e) => {
            // Connection to server lost
            return State::DeviceDiscovery
        }
    };

}



fn get_board_id_from_command(command: command::Command) -> Option<u32> {
    match command.command {
        command::mod_Command::OneOfcommand::set_led(set_led) => {
            return set_led.led.unwrap().board_id?;
        },
        command::mod_Command::OneOfcommand::click_valve(click_valve) => {
            return click_valve.valve.unwrap().board_id?;
        },
        _ => {
            return None
        }
    }
}