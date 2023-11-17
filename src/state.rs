

use fs_protobuf_rust::compiled::mcfs::board;
use std::collections::HashMap;

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



pub struct State {
    pub sensor_data: HashMap<String, f64>,
    pub channel_mapping: HashMap<ChannelIdentifier, String>
}