use crate::common::*;

pub enum Message {
    DiscoveryRequest(Address),
    DiscoveryAcknowledge(Address),
    GameUpdate(GameUpdateData),
}

pub fn get_message_id(message: &Message) -> u8 {
    match message {
        Message::DiscoveryRequest(_) => 0,
        Message::DiscoveryAcknowledge(_) => 1,
        Message::GameUpdate(_) => 2,
    }
}

pub fn message_from_data(data: &[u8]) -> Result<Message, ()> {
    match data[0] {
        0 => Ok(Message::DiscoveryRequest(data[1])),
        1 => Ok(Message::DiscoveryAcknowledge(data[1])),
        2 => Ok(Message::GameUpdate(GameUpdateData)),
        _ => Err(()),
    }
}

pub struct GameUpdateData;
