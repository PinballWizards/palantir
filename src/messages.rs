use serde::{Deserialize, Serialize};

pub trait Message {}

pub enum SlaveMessage {
    Broadcast(Broadcast),
    UpdateRequest(UpdateRequest),
}

#[derive(Deserialize, Serialize)]
pub struct Broadcast {
    game_state: u32,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateRequest;

impl Message for UpdateRequest {}

#[derive(Deserialize, Serialize)]
pub struct SolenoidUpdate {
    input_states: [bool; 12],
    output_states: [bool; 6],
}
