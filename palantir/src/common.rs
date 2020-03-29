pub type Address = u8;
pub type SlaveAddresses = [Address; 7];

pub const MASTER_ADDRESS: Address = 1;

/// This is the maximum message length including address and crc bytes.
pub const MAX_MESSAGE_LEN: usize = 64;
