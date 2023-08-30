use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum FromClientMessage {
    Ping, // Asks for Pong
    Screen(Vec<u8>),
    //ChunkSize(usize), // Used when a message is potentially to big - not needed in websockets, yay
    ScreenSize((u32, u32)), // x, y
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FromServerMessage {
    Pong, // Answers for Ping
    Click(u16, u16), // Click at this location x / y
    RequestScreen,
}
