

use fs_protobuf_rust::compiled::mcfs::{board, mapping};
use std::collections::HashMap;

pub struct State {
    pub sensor_data: HashMap<String, f64>,
    pub channel_mapping: HashMap<ChannelIdentifier, String>
}

impl State {
    pub fn new() -> State {
        State {
            sensor_data: HashMap::new(),
            channel_mapping: HashMap::new(),
        }
    }

    pub fn get_sensor_data(&self, sensor_name: &str) -> Option<&f64> {
        self.sensor_data.get(sensor_name)
    }

    pub fn set_mappings(&mut self, mapping: mapping::Mapping) {
        self.channel_mapping.clear();
        for mapping::ChannelMapping {name, channel_identifier} in mapping.channel_mappings {
            if let Some(channel_identifier) = channel_identifier {
                self.channel_mapping.insert(ChannelIdentifier::from_proto(&channel_identifier), name.to_string());
            }
        }
        println!("Channel mapping: {:?}", self.channel_mapping);
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