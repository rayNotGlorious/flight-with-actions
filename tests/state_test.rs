use std::collections::HashMap;
use fc::state;
use fs_protobuf_rust::compiled::mcfs::board;

#[test]
fn test_channel_mappings() {
    let mut state = state::State {
        sensor_data: HashMap::new(),
        channel_mapping: HashMap::new(),
    };

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