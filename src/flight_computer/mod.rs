use std::{collections::HashMap, net::{IpAddr, SocketAddr, UdpSocket}, io::{Read, self}};
use quick_protobuf::{deserialize_from_slice, Error};

use crate::discovery::get_ips;
use std::net::TcpStream;

use fs_protobuf_rust::compiled::mcfs::{core, command};

const SERVER_ADDR: &str = "fs-server-02.local";
const HOSTNAMES: [&str; 3] = [SERVER_ADDR, "fs-sam-01.local", "fs-sam-02.local"];

pub struct Data {
    ip_addresses: HashMap<String, Option<IpAddr>>,
    board_ids: HashMap<u32, String>,
    server: Option<TcpStream>,
    state_num: u32,
}

impl Data {
    pub fn new() -> Data {
        let mut board_ids = HashMap::new();
        board_ids.insert(1, "fs-sam-01.local".to_string());
        board_ids.insert(2, "fs-sam-02.local".to_string());
        board_ids.insert(3, "fs-sam-03.local".to_string());
        board_ids.insert(4, "fs-sam-04.local".to_string());
        
        Data {
            ip_addresses: HashMap::new(),
            board_ids: board_ids,
            server: None,
            state_num: 0,
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

        if data.state_num % 1000000 == 0 {
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
            data.server.as_ref().unwrap().set_nonblocking(false).expect("set_nonblocking call failed");
            return State::HandleCommands
        },
        Err(_e) => {
            return State::DeviceDiscovery
        }
    }
}

fn handle_commands(data: &mut Data) -> State {

    // receive command from server
    let mut buf = vec![0; 65536];


    match data.server.as_mut().unwrap().read(&mut buf) {
        Ok(bytes) => {

            
            println!("\n\n\nreceived {} bytes", bytes);


            let deserialized: core::Message= deserialize_from_slice(&buf).unwrap();

            println!("{:?}", deserialized);

            match deserialized.content {
                core::mod_Message::OneOfcontent::command(command) => {
                    handle_command(data, command)
                },
                core::mod_Message::OneOfcontent::mapping(mapping) => {
                    update_mapping(data, mapping);
                    return State::HandleCommands
                },
                _ => {
                    return State::HandleCommands
                }
            }
            
        },
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
            return State::HandleCommands
        }
        Err(_e) => {
            return State::HandleCommands
        }
    };
    return State::HandleCommands

}



fn get_board_id_from_command(command: command::Command) -> Option<u32> {
    match command.command {
        command::mod_Command::OneOfcommand::set_led(set_led) => {
            return Some(set_led.led.unwrap().board_id);
        },
        command::mod_Command::OneOfcommand::click_valve(click_valve) => {
            return Some(click_valve.valve.unwrap().board_id);
        },
        _ => {
            return None
        }
    }
}

fn handle_command(data: &mut Data, command: command::Command) -> State {
    if let Some(board_id) = get_board_id_from_command(command) {
        println!("board_id: {}", board_id);
        if let Some(hostname) = data.board_ids.get(&board_id) {
            println!("hostname: {}", hostname);
            if let Some(ip) = data.ip_addresses.get(hostname) {
                match ip {
                    Some(ipv4_addr) => {
                        let socket_addr = SocketAddr::new(*ipv4_addr, 8378);
                        let socket = UdpSocket::bind("0.0.0.0:9572").expect("couldn't bind to address");
                        socket.connect(socket_addr).expect("connect function failed");
                        socket.send(&buf[..bytes]).expect("couldn't send message");
                        println!("Sent command to {}", hostname);
                        return State::HandleCommands
                    }
                    None => {
                        println!("Could not find {} on local network", hostname);
                        return State::HandleCommands
                    }
                }
            } else {
                println!("Could not find {} on local network", hostname);
                return State::HandleCommands
            }
        } else {
            println!("Board {} not mapped", board_id);
            return State::HandleCommands
        }
    } else {
        println!("no board_id");
    }
    return State::HandleCommands
}