use rkyv::{Archive, Deserialize, Serialize};
#[derive(Archive, Deserialize, Serialize)]
pub struct SendMessage {
    msg: String,
}
#[derive(Archive, Deserialize, Serialize)]
pub enum Client {
    SendMessage(SendMessage),
}
