use std::collections::HashMap;
use fc::state;
use fs_protobuf_rust::compiled::mcfs::board;

#[test]
fn test_channel_mappings() {
    let mut state = state::State::new();

    state.channel_mapping.insert(
        state::ChannelIdentifier::new(1,  board::ChannelType::CURRENT_LOOP as u32, 1),
        "PT1".to_string(),
    );

    state.channel_mapping.insert(
        state::ChannelIdentifier::new(1,  board::ChannelType::CURRENT_LOOP as u32, 2),
        "PT2".to_string(),
    );

    state.channel_mapping.insert(
        state::ChannelIdentifier::new(1,  board::ChannelType::CURRENT_LOOP as u32, 3),
        "PT3".to_string(),
    );

    assert_eq!(state.channel_mapping.len(), 3);
    assert_eq!(state.channel_mapping.get(&state::ChannelIdentifier::new(1,  board::ChannelType::CURRENT_LOOP as u32, 1)), Some(&"PT1".to_string()));
    assert_eq!(state.channel_mapping.get(&state::ChannelIdentifier::new(1,  board::ChannelType::CURRENT_LOOP as u32, 2)), Some(&"PT2".to_string()));
    assert_eq!(state.channel_mapping.get(&state::ChannelIdentifier::new(1,  board::ChannelType::CURRENT_LOOP as u32, 3)), Some(&"PT3".to_string()));
}

#[test]
fn test_mappings_from_proto() {
    let mut state = state::State::new();

    let channel_identifier = board::ChannelIdentifier {
        board_id: 1,
        channel_type: board::ChannelType::CURRENT_LOOP,
        channel: 1,
    };

    state.channel_mapping.insert(
        state::ChannelIdentifier::from_proto(&channel_identifier),
        "PT1".to_string(),
    );

    assert_eq!(state.channel_mapping.len(), 1);
    assert_eq!(state.channel_mapping.get(&state::ChannelIdentifier::new(1,  board::ChannelType::CURRENT_LOOP as u32, 1)), Some(&"PT1".to_string()));
}

#[test]
fn test_state_data() {
    let mut state = state::State::new();

    state.sensor_data.insert("PT1".to_string(), 1.0);
    state.sensor_data.insert("PT2".to_string(), 2.0);
    state.sensor_data.insert("PT3".to_string(), 3.0);

    assert!(state.sensor_data.len() == 3);
    assert!(state.sensor_data.get("PT1").unwrap() == &1.0);
    assert!(state.sensor_data.get("PT2").unwrap() == &2.0);
    assert!(state.sensor_data.get("PT3").unwrap() == &3.0);

    state.sensor_data.insert("PT1".to_string(), 5.0);
    assert!(state.sensor_data.get("PT1").unwrap() == &5.0);


}