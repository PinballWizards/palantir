use crate::common::*;

pub enum Message {
    DiscoveryRequest(DiscoveryRequestData),
    DiscoveryAcknowledge(DiscoveryAcknowledgeData),
    GameUpdate(GameUpdateData),
}

pub struct DiscoveryRequestData {
    address: Address,
}

impl DiscoveryRequestData {
    pub fn new(target_address: Address) -> Self {
        DiscoveryRequestData {
            address: target_address,
        }
    }

    pub fn target_address(&self) -> Address {
        self.address
    }

    pub fn from_slice(data: &[u8]) -> Result<Self, ()> {
        if data.len() >= 1 {
            return Ok(DiscoveryRequestData { address: data[0] });
        }
        Err(())
    }

    pub fn to_array(&self) -> [u8; 1] {
        let mut ret = [0u8; 1];
        ret[0] = self.address;
        ret
    }
}

pub struct DiscoveryAcknowledgeData {
    address: Address,
}

impl DiscoveryAcknowledgeData {
    pub fn new(responder_address: Address) -> Self {
        DiscoveryAcknowledgeData {
            address: responder_address,
        }
    }

    pub fn responder_address(&self) -> Address {
        self.address
    }

    pub fn from_slice(data: &[u8]) -> Result<Self, ()> {
        if data.len() >= 1 {
            return Ok(DiscoveryAcknowledgeData { address: data[0] });
        }
        Err(())
    }

    pub fn to_array(&self) -> [u8; 1] {
        let mut ret = [0u8; 1];
        ret[0] = self.address;
        ret
    }
}

pub struct GameUpdateData {
    some_info: u32,
}

pub fn get_message_id(message: &Message) -> u8 {
    match message {
        Message::DiscoveryRequest(_) => 0,
        Message::DiscoveryAcknowledge(_) => 1,
        Message::GameUpdate(_) => 2,
    }
}

pub fn message_from_data(data: &[u8]) -> Result<Message, ()> {
    // First byte is ID
    match data[0] {
        0 => {
            let data = match DiscoveryRequestData::from_slice(&data[1..]) {
                Ok(v) => v,
                _ => return Err(()),
            };
            Ok(Message::DiscoveryRequest(data))
        }
        1 => {
            let data = match DiscoveryAcknowledgeData::from_slice(&data[1..]) {
                Ok(v) => v,
                _ => return Err(()),
            };
            Ok(Message::DiscoveryAcknowledge(data))
        }
        _ => Err(()),
    }
}

pub fn data_from_message(message: &Message, buf: &mut [u8; MAX_DATA_LEN]) -> Result<usize, ()> {
    let sliced_len = match message {
        Message::DiscoveryRequest(data) => {
            // Set first byte as ID
            buf[0] = 0;
            let serialized_data = data.to_array();
            buf[1..2].copy_from_slice(&serialized_data);
            // 1+ is necessary here due to ID byte
            1 + serialized_data.len()
        }
        Message::DiscoveryAcknowledge(data) => {
            // Set first byte as ID
            buf[0] = 1;
            let serialized_data = data.to_array();
            buf[1..2].copy_from_slice(&serialized_data);
            // 1+ is necessary here due to ID byte
            1 + serialized_data.len()
        }
        _ => return Err(()),
    };

    Ok(sliced_len)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_discovery_request() {
        let msg = Message::DiscoveryRequest(DiscoveryRequestData::new(1));
        let mut buf = [0u8; MAX_DATA_LEN];
        let data_size = match data_from_message(&msg, &mut buf) {
            Ok(len) => len,
            _ => panic!("Could not gen data array from message"),
        };

        let lazarus = match message_from_data(buf.split_at(data_size).0) {
            Ok(Message::DiscoveryRequest(data)) => data,
            _ => panic!("could not recreate message from data"),
        };

        match msg {
            Message::DiscoveryRequest(data) => {
                assert_eq!(data.target_address(), lazarus.target_address())
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_discovery_acknowledge() {
        let msg = Message::DiscoveryAcknowledge(DiscoveryAcknowledgeData::new(1));
        let mut buf = [0u8; MAX_DATA_LEN];
        let data_size = match data_from_message(&msg, &mut buf) {
            Ok(len) => len,
            _ => panic!("Could not gen data array from message"),
        };

        let lazarus = match message_from_data(buf.split_at(data_size).0) {
            Ok(Message::DiscoveryAcknowledge(data)) => data,
            _ => panic!("could not recreate message from data"),
        };

        match msg {
            Message::DiscoveryAcknowledge(data) => {
                assert_eq!(data.responder_address(), lazarus.responder_address())
            }
            _ => panic!(),
        }
    }
}
