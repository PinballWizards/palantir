#![cfg_attr(not(feature = "std"), no_std)]

mod common;
pub use common::*;

#[cfg(feature = "feather_bus")]
pub mod feather_bus;
pub mod messages;
mod parser;

use messages::*;
use nb;
use parser::Parser;

pub trait Bus {
    type Error;

    fn send(&mut self, data: &[u16]);
    fn read(&mut self) -> nb::Result<u16, Self::Error>;
}

pub enum Error {
    NotMaster,
    SendToSelf,
    InvalidDiscoveryAck,
    /// Slave received a different message when it was anticipating a discovery request.
    InvalidDiscoveryReq,
    Other,
}

pub struct Palantir<B: Bus> {
    parser: Parser,
    address: Address,
    bus: B,
    slaves: Option<SlaveAddresses>,
    loopback: bool,
}

impl<B: Bus> Palantir<B> {
    pub fn new_slave(device_address: Address, bus: B) -> Self {
        Palantir {
            parser: Parser::new(device_address),
            address: device_address,
            bus,
            slaves: None,
            loopback: false,
        }
    }

    pub fn new_master(slaves: SlaveAddresses, bus: B) -> Self {
        Palantir {
            parser: Parser::new(MASTER_ADDRESS),
            address: MASTER_ADDRESS,
            bus,
            slaves: Some(slaves),
            loopback: false,
        }
    }

    fn wait_for_discovery_ack(&mut self, address: Address) -> Result<(), Error> {
        let msg: Message = loop {
            match self.poll() {
                Some(msg) => break msg,
                None => continue,
            }
        };

        let _valid_data = DiscoveryAcknowledgeData::new(address);

        match msg {
            Message::DiscoveryAcknowledge(_valid_data) => Ok(()),
            _ => Err(Error::InvalidDiscoveryAck),
        }
    }

    fn wait_for_discovery_request(&mut self) -> Result<(), Error> {
        Ok(())
    }

    /// This should only be called by the master device at startup!
    pub fn discover_devices(&mut self) -> Result<(), Error> {
        for slave in self.slaves.unwrap().iter() {
            let message = Message::DiscoveryRequest(DiscoveryRequestData::new(*slave));
            match self.send(*slave, &message) {
                Err(_) => return Err(Error::Other),
                _ => (),
            };
            self.wait_for_discovery_ack(*slave)?;
        }
        Ok(())
    }

    /// This should be called only by slave devices at startup.
    pub fn discovery_mode(&mut self) -> Result<(), Error> {
        self.wait_for_discovery_request()
    }

    pub fn send(&mut self, address: Address, message: &Message) -> Result<(), Error> {
        if !self.loopback && address == self.address {
            return Err(Error::SendToSelf);
        }

        let mut data = [0u8; MAX_DATA_LEN];
        let data_len = match messages::data_from_message(message, &mut data) {
            Ok(len) => len,
            _ => return Err(Error::Other),
        };
        let mut payload = [0u16; MAX_MESSAGE_LEN];
        payload[0] = (1 << 8) | address as u16;
        payload[1] = data_len as u16;
        for (place, data) in payload[2..].iter_mut().zip(data.iter()) {
            *place = *data as u16;
        }

        // +2 here for the address and data length bytes
        self.bus.send(payload.split_at(2 + data_len).0);

        // let payload = match message.to_payload() {
        //     Ok(v) => v,
        //     Err(_) => return Err(Error::Other),
        // };
        // self.bus.send(&match self.transport.send(address, payload) {
        //     Ok(data) => data,
        //     _ => return Err(Error::Other),
        // });
        Ok(())
    }

    pub fn poll(&mut self) -> Option<Message> {
        let data = match self.bus.read() {
            Ok(val) => val,
            _ => return None,
        };
        self.parser.ingest(data);
        self.parser.poll_message()
    }

    pub fn ingest(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    impl<B: Bus> Palantir<B> {
        fn new_loopback(address: Address, bus: B) -> Self {
            Palantir {
                parser: Parser::new(address),
                address,
                bus,
                slaves: None,
                loopback: true,
            }
        }
    }

    struct MockBus {
        buf: VecDeque<u16>,
    }

    impl MockBus {
        fn new() -> Self {
            Self {
                buf: VecDeque::new(),
            }
        }
    }

    impl Bus for MockBus {
        type Error = ();
        fn send(&mut self, data: &[u16]) {
            for x in data {
                self.buf.push_back(*x);
            }
        }

        fn read(&mut self) -> nb::Result<u16, Self::Error> {
            match self.buf.pop_front() {
                Some(val) => Ok(val),
                None => Err(nb::Error::Other(())),
            }
        }
    }

    fn get_mocked_slave(address: Address) -> Palantir<MockBus> {
        Palantir::new_loopback(address, MockBus::new())
    }

    #[test]
    fn echo_bus() {
        let mut bus = MockBus::new();
        bus.send(&[5]);
        match bus.read() {
            Ok(v) => assert_eq!(v, 5),
            Err(_) => panic!("did not get same value back"),
        }
    }

    #[test]
    fn discovery_req_transmit() {
        let mut bus = MockBus::new();
        let mut palantir = Palantir::new_loopback(MASTER_ADDRESS, bus);

        let msg = Message::DiscoveryRequest(DiscoveryRequestData::new(9));

        palantir.send(MASTER_ADDRESS, &msg);

        let mut msg: Option<Message> = None;

        for _ in 0..MAX_MESSAGE_LEN {
            msg = palantir.poll();
            if msg.is_some() {
                break;
            }
        }
        match msg {
            Some(Message::DiscoveryRequest(data)) => assert_eq!(data.target_address(), 9),
            _ => panic!("got something that wasnt a discovery request"),
        }
    }
}
