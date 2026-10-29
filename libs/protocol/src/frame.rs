pub const S_FRAME: usize = 1024 * 1024; //1m
pub struct Frame {
    pub size: u32,
    pub payload: Vec<u8>,
}
impl Frame {
    pub fn from(size: u32, payload: Vec<u8>) -> Self {
        Self { size, payload }
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();
        v.extend_from_slice(&self.size.to_be_bytes());
        v.extend_from_slice(&self.payload);
        v
    }
}
