use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum FromClientMessage {
    Ping, // Asks for Pong
}

#[derive(Serialize, Deserialize)]
pub enum FromServerMessage {
    Pong, // Answers for Ping
    Click(u16, u16), // Click at this location x / y
}
