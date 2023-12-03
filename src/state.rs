use fs_protobuf_rust::compiled::mcfs::{board, mapping, data};
use std::{collections::HashMap, sync::{Arc, RwLock}, net::IpAddr, alloc::System, time::SystemTime};
use lazy_static::lazy_static;
use crate::discovery::get_ips;
use common::VehicleState;

lazy_static! {
    static ref STATE: Arc<RwLock<State>> = Arc::new(RwLock::new(State::new()));
}

pub fn get_state() -> Arc<RwLock<State>> {
    STATE.clone()
}

pub fn get_vehicle_state() -> VehicleState {
    let state = STATE.read().unwrap();
    state.vehicle_state.clone()
}

pub fn read_sensor(sensor_name: &str) -> Option<f64> {
    let state = STATE.read().unwrap();
    if let Some(sensor_data) = state.read_sensor(sensor_name) {
        Some(*sensor_data)
    } else {
        None
    }
}

pub fn get_valve(valve: &str) -> Option<board::ChannelIdentifier> {
    let state = STATE.read().unwrap();
    if let Some(valve) = state.valve_mapping.get(valve) {
        Some(valve.clone())
    } else {
        None
    }
}

pub fn get_hostname_from_id(board_id: u32) -> Option<String>{
    let state = STATE.read().unwrap();
    return state.board_ids.get(&board_id).cloned();
}

pub fn get_ip_from_hostname(hostname: &str) -> Option<IpAddr> {
    let state = STATE.read().unwrap();
    if let Some(ip) = state.ip_addresses.get(hostname) {
        Some(ip.clone())
    } else {
        None
    }
}

pub fn set_mappings(mapping: mapping::Mapping) {
    let mut state = STATE.write().unwrap();
    state.set_mappings(mapping);
}

pub fn insert_sensor_data(data: data::Data) {
    let mut state = STATE.write().unwrap();
    for data_point in data.channel_data {
        let channel_identifier = ChannelIdentifier::from_proto(&data_point.channel.unwrap());
        match data_point.data_points {
            data::mod_ChannelData::OneOfdata_points::f64_array(data) => {
                state.insert_data_from_channel_identifier(&channel_identifier, data.data[0]);
            }
            data::mod_ChannelData::OneOfdata_points::f32_array(data) => {
                state.insert_data_from_channel_identifier(&channel_identifier, data.data[0] as f64);
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct State {
    pub sensor_data: HashMap<String, f64>,
    pub channel_mapping: HashMap<ChannelIdentifier, String>,
    pub valve_mapping: HashMap<String, board::ChannelIdentifier>,
    pub ip_addresses: HashMap<String, IpAddr>,
    pub board_ids: HashMap<u32, String>,
    pub vehicle_state: VehicleState,
    pub start_time: SystemTime,

}

impl State {
    pub fn new() -> State {
        let mut board_ids = HashMap::new();
        board_ids.insert(1, "sam-01.local".to_string());
        board_ids.insert(2, "sam-02.local".to_string());
        board_ids.insert(3, "sam-03.local".to_string());
        board_ids.insert(4, "sam-04.local".to_string());

        const HOSTNAMES: [&str; 2] = ["sam-01.local", "sam-02.local"];
        let ip_addresses = get_ips(&HOSTNAMES);




        State {
            sensor_data: HashMap::new(),
            channel_mapping: HashMap::new(),
            valve_mapping: HashMap::new(),
            ip_addresses: ip_addresses,
            board_ids: board_ids,
            vehicle_state: VehicleState::new(),
            start_time: SystemTime::now(),
        }
    }

    pub fn read_sensor(&self, sensor_name: &str) -> Option<&f64> {
        self.sensor_data.get(sensor_name)
    }

    pub fn set_mappings(&mut self, mapping: mapping::Mapping) {
        self.channel_mapping.clear();
        for mapping::ChannelMapping {name, channel_identifier} in mapping.channel_mappings {
            if let Some(channel_identifier) = channel_identifier {
                match channel_identifier.channel_type {
                    board::ChannelType::VALVE => {
                        self.valve_mapping.insert(name.to_string(), channel_identifier);
                    }
                    _ => {
                        self.channel_mapping.insert(ChannelIdentifier::from_proto(&channel_identifier), name.to_string());
                    }
                }
            }
        }
        println!("Channel mapping: {:?}", self.channel_mapping);
    }

    pub fn get_sensor_name(&self, channel_identifier: &ChannelIdentifier) -> Option<&String> {
        self.channel_mapping.get(channel_identifier).clone()
    }

    pub fn insert_data(&mut self, sensor_name: &str, data: f64) {
        self.sensor_data.insert(sensor_name.to_string(), data);
        self.vehicle_state.sensor_readings.insert(sensor_name.to_string(), common::Unit::Volts(data));
        self.vehicle_state.update_times.insert(sensor_name.to_string(), SystemTime::now().duration_since(self.start_time).unwrap().as_micros() as f64);
    }

    pub fn insert_data_from_channel_identifier(&mut self, channel_identifier: &ChannelIdentifier, data: f64) {
        if let Some(sensor_name) = self.get_sensor_name(channel_identifier) {
            let sensor_name = sensor_name.clone();
            self.insert_data(sensor_name.as_str(), data);
        }
    }
}
// Internal representation of channel identifier that is hashable,
// since we cannot modify the protobuf generated code
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ChannelIdentifier {
    pub board_id: u32,
    pub channel_type: u32,
    pub channel: u32,
}

impl ChannelIdentifier {
    pub fn new(board_id: u32, channel_type: u32, channel: u32) -> ChannelIdentifier {
        ChannelIdentifier {
            board_id,
            channel_type,
            channel,
        }
    }

    pub fn from_proto(proto: &board::ChannelIdentifier) -> ChannelIdentifier {
        ChannelIdentifier {
            board_id: proto.board_id,
            channel_type: proto.channel_type as u32,
            channel: proto.channel,
        }
    }
    
}