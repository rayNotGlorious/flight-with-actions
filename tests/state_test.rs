use std::collections::HashMap;
use fc::{state, sequences::run_python_sequence};
use fs_protobuf_rust::compiled::mcfs::{board, data, mapping};

#[test]
fn test_sequence() {
    let mappings = mapping::Mapping {
        channel_mappings: vec![
            mapping::ChannelMapping {
                name: std::borrow::Cow::Borrowed("pt1"),
                channel_identifier: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::CURRENT_LOOP,
                    channel: 0
                }),
            },
            mapping::ChannelMapping {
                name: std::borrow::Cow::Borrowed("pt2"),
                channel_identifier: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::CURRENT_LOOP,
                    channel: 1
                }),
            },
            mapping::ChannelMapping {
                name: std::borrow::Cow::Borrowed("pt3"),
                channel_identifier: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::CURRENT_LOOP,
                    channel: 2
                }),
            },
            mapping::ChannelMapping {
                name: std::borrow::Cow::Borrowed("pt4"),
                channel_identifier: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::CURRENT_LOOP,
                    channel: 3
                }),
            },
            mapping::ChannelMapping {
                name: std::borrow::Cow::Borrowed("pt5"),
                channel_identifier: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::CURRENT_LOOP,
                    channel: 4
                }),
            },
            mapping::ChannelMapping {
                name: std::borrow::Cow::Borrowed("pt6"),
                channel_identifier: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::CURRENT_LOOP,
                    channel: 5
                }),
            },
            mapping::ChannelMapping {
                name: std::borrow::Cow::Borrowed("valve1"),
                channel_identifier: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::VALVE,
                    channel: 0
                }),
            },
            mapping::ChannelMapping {
                name: std::borrow::Cow::Borrowed("valve2"),
                channel_identifier: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::VALVE,
                    channel: 1
                }),
            },
        ],
    };

    let data = data::Data {
        channel_data: vec![
            data::ChannelData {
                channel: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::CURRENT_LOOP,
                    channel: 0
                }),
                timestamp: None,
                micros_offsets: vec![0],
                data_points: data::mod_ChannelData::OneOfdata_points::f64_array(data::F64Array {
                    data: std::borrow::Cow::Borrowed(&[1.0])
                })
            },
            data::ChannelData {
                channel: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::CURRENT_LOOP,
                    channel: 1
                }),
                timestamp: None,
                micros_offsets: vec![0],
                data_points: data::mod_ChannelData::OneOfdata_points::f64_array(data::F64Array {
                    data: std::borrow::Cow::Borrowed(&[2.0])
                })
            },
            data::ChannelData {
                channel: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::CURRENT_LOOP,
                    channel: 2
                }),
                timestamp: None,
                micros_offsets: vec![0],
                data_points: data::mod_ChannelData::OneOfdata_points::f64_array(data::F64Array {
                    data: std::borrow::Cow::Borrowed(&[3.0])
                })
            },
            data::ChannelData {
                channel: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::CURRENT_LOOP,
                    channel: 3
                }),
                timestamp: None,
                micros_offsets: vec![0],
                data_points: data::mod_ChannelData::OneOfdata_points::f64_array(data::F64Array {
                    data: std::borrow::Cow::Borrowed(&[4.0])
                })
            },
            data::ChannelData {
                channel: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::CURRENT_LOOP,
                    channel: 4
                }),
                timestamp: None,
                micros_offsets: vec![0],
                data_points: data::mod_ChannelData::OneOfdata_points::f64_array(data::F64Array {
                    data: std::borrow::Cow::Borrowed(&[5.0])
                })
            },
            data::ChannelData {
                channel: Some(board::ChannelIdentifier {
                    board_id: 1,
                    channel_type: board::ChannelType::CURRENT_LOOP,
                    channel: 5
                }),
                timestamp: None,
                micros_offsets: vec![0],
                data_points: data::mod_ChannelData::OneOfdata_points::f64_array(data::F64Array {
                    data: std::borrow::Cow::Borrowed(&[6.0])
                })
            }
        ]
    };

    state::set_mappings(mappings);
    state::insert_sensor_data(data);

    let sequence = "from libseq import *; print(pt1.read()); wait_for(5 * s); valve1.open();";
    assert_eq!(state::read_sensor("pt1"), Some(1.0));
    assert_eq!(state::read_sensor("pt2"), Some(2.0));
    assert_eq!(state::read_sensor("pt3"), Some(3.0));
    assert_eq!(state::read_sensor("pt4"), Some(4.0));
    assert_eq!(state::read_sensor("pt5"), Some(5.0));
    assert_eq!(state::read_sensor("pt6"), Some(6.0));
    assert_eq!(state::read_sensor("pt7"), None);
    

    println!("\n\n\n\n\n");
    run_python_sequence(&sequence.to_string());
    println!("\n\n\n\n\n");
}