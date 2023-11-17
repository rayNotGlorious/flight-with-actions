use std::{collections::HashMap, net::{IpAddr, SocketAddr, UdpSocket}, io::{Read, self}, sync::{Arc, RwLock}};
use quick_protobuf::{deserialize_from_slice, Error};

use crate::{discovery::get_ips, state::State, sequences::run_python_sequence};
use std::net::TcpStream;

use fs_protobuf_rust::compiled::mcfs::{core, command};


const SERVER_ADDR: &str = "192.168.0.176";
const HOSTNAMES: [&str; 3] = [SERVER_ADDR, "fs-sam-01.local", "fs-sam-02.local"];

pub struct FlightComputer {
    ip_addresses: HashMap<String, Option<IpAddr>>,
    state: Arc<RwLock<State>>,
    board_ids: HashMap<u32, String>,
    server: Option<TcpStream>,
    fc_state: FCState,
    fc_state_num: u32,
    sequence: String,
}

impl FlightComputer {
    pub fn new(state: Arc<RwLock<State>>) -> FlightComputer {
        let mut board_ids = HashMap::new();
        board_ids.insert(1, "fs-sam-01.local".to_string());
        board_ids.insert(2, "fs-sam-02.local".to_string());
        board_ids.insert(3, "fs-sam-03.local".to_string());
        board_ids.insert(4, "fs-sam-04.local".to_string());
        
        FlightComputer {
            ip_addresses: HashMap::new(),
            state: state,
            board_ids,
            server: None,
            fc_state: FCState::Init,
            fc_state_num: 0,
            sequence: "print('hello')".to_string(),
        }
    }

    pub fn run(&mut self) {
        loop {

            if self.fc_state_num % 1000000 == 0 {
                println!("{:?} {}", self.fc_state, self.fc_state_num);
            }

            self.fc_state_num += 1;

            self.fc_state = match self.fc_state {
                FCState::Init => {
                    self.init()
                }
                FCState::DeviceDiscovery => {
                    self.device_discovery()
                }
                FCState::ConnectToServer => {
                    self.connect_to_server()
                }
                FCState::HandleCommands => {
                    self.handle_commands()
                }
                FCState::RunSequence => {
                    self.run_sequence()
                }
            };
        }
    }

    fn init(&mut self) -> FCState {
        FCState::RunSequence
    }

    fn device_discovery(&mut self) -> FCState {
        self.ip_addresses = get_ips(&HOSTNAMES);
        if let Some(ip) = self.ip_addresses.get(SERVER_ADDR) {
            match ip {
                Some(_ipv4_addr) => {
                    FCState::ConnectToServer
                },
                None => {
                    FCState::DeviceDiscovery
                }
            }
        } else {
            FCState::DeviceDiscovery
        }
    }

    fn connect_to_server(&mut self) -> FCState {
        let server_addr = self.ip_addresses.get(SERVER_ADDR).unwrap().unwrap();
        let socket_addr = SocketAddr::new(server_addr, 5025);
        match TcpStream::connect(socket_addr) {
            Ok(stream) => {
                self.server = Some(stream);
                self.server.as_ref().unwrap().set_nonblocking(false).expect("set_nonblocking call failed");
                FCState::HandleCommands
            },
            Err(_e) => {
                FCState::DeviceDiscovery
            }
        }
    }

    fn handle_commands(&mut self) -> FCState {
        let mut buf = vec![0; 65536];


        match self.server.as_mut().unwrap().read(&mut buf) {
            Ok(bytes) => {
                println!("\n\n\nreceived {} bytes", bytes);
                let deserialized: core::Message= deserialize_from_slice(&buf).unwrap();
                println!("{:?}", deserialized);
                match deserialized.content {
                    core::mod_Message::OneOfcontent::command(command) => {
                        self.handle_command(command, &buf, bytes);
                    },
                    core::mod_Message::OneOfcontent::mapping(mapping) => {
                        println!("mapping: {:?}", mapping);
                        self.state.write().unwrap().set_mappings(mapping);
                    },
                    core::mod_Message::OneOfcontent::sequence(sequence) => {
                        println!("sequence: {:?}", sequence);
                        self.sequence = sequence.script.to_string();
                        return FCState::HandleCommands
                    },
                    _ => {
                        return FCState::HandleCommands
                    }
                }
                
            },
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                return FCState::HandleCommands
            }
            Err(_e) => {
                return FCState::HandleCommands
            }
        };
        FCState::HandleCommands
    }

    fn run_sequence(&mut self) -> FCState {
        run_python_sequence(&self.sequence);
        println!("running sequence: {}", self.sequence);
        FCState::DeviceDiscovery
    }

    fn handle_command(&mut self, command: command::Command, buf: &[u8], bytes: usize) {
        if let Some(board_id) = get_board_id_from_command(command) {
            println!("board_id: {}", board_id);
            if let Some(hostname) = self.board_ids.get(&board_id) {
                println!("hostname: {}", hostname);
                if let Some(ip) = self.ip_addresses.get(hostname) {
                    match ip {
                        Some(ipv4_addr) => {
                            let socket_addr = SocketAddr::new(*ipv4_addr, 8378);
                            let socket = UdpSocket::bind("0.0.0.0:9572").expect("couldn't bind to address");
                            socket.connect(socket_addr).expect("connect function failed");
                            socket.send(&buf[..bytes]).expect("couldn't send message");
                            println!("Sent command to {}", hostname);
                        }
                        None => {
                            println!("Could not find {} on local network", hostname);
                        }
                    }
                } else {
                    println!("Could not find {} on local network", hostname);
                }
            } else {
                println!("Board {} not mapped", board_id);
            }
        } else {
            println!("no board_id");
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum FCState {
    Init,
    DeviceDiscovery,
    ConnectToServer,
    HandleCommands,
    RunSequence,
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