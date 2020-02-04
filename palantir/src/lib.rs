#![no_std]

#[macro_use]
extern crate bitfield;

pub mod messages;

mod parser;
mod transport;

use core::convert::TryFrom;
use messages::*;
use nom::number::complete::le_u8;
use transport::{Address, Response, Transport, MASTER_ADDRESS};

pub trait Bus {
    fn send(&mut self, data: &[u16]);
    fn read(&mut self) -> u16;
}

pub struct Palantir<B: Bus> {
    transport: Transport,
    bus: B,
}

impl<B: Bus> Palantir<B> {
    pub fn new(device_address: Address, bus: B) -> Self {
        Palantir {
            transport: match device_address {
                MASTER_ADDRESS => Transport::new_master(),
                _ => Transport::new_slave(device_address),
            },
            bus: bus,
        }
    }

    pub fn send<M: Message>(&mut self, message: M) -> Result<(), ()> {
        let payload = message.to_payload()?;
        self.bus.send(&match self.transport.send(payload) {
            Ok(data) => data,
            _ => return Err(()),
        });
        Ok(())
    }

    pub fn poll(&mut self) -> Option<ReceivedMessage> {
        let frame = self.transport.parse_data_buffer()?;

        let (i, msg_id) = match le_u8::<()>(frame.app_data()) {
            Ok(v) => v,
            Err(_) => return None,
        };

        let id = match MessageID::try_from(msg_id) {
            Ok(id) => id,
            _ => return None,
        };

        match id {
            MessageID::Broadcast => match Broadcast::try_from(i) {
                Ok(msg) => Some(ReceivedMessage::Broadcast(msg)),
                _ => None,
            },
            MessageID::DiscoveryRequest => match DiscoveryRequest::try_from(i) {
                Ok(msg) => Some(ReceivedMessage::DiscoveryRequest(msg)),
                _ => None,
            },
            MessageID::DiscoveryAck => match DiscoveryAck::try_from(i) {
                Ok(msg) => Some(ReceivedMessage::DiscoveryAck(msg)),
                _ => None,
            },
            MessageID::UpdateRequest => match UpdateRequest::try_from(i) {
                Ok(msg) => Some(ReceivedMessage::UpdateRequest(msg)),
                _ => None,
            },
            MessageID::SolenoidUpdate => match SolenoidUpdate::try_from(i) {
                Ok(msg) => Some(ReceivedMessage::SolenoidUpdate(msg)),
                _ => None,
            },
        }
    }

    /// This function must ONLY be called when a bus read will succeed and return valid data.
    pub fn ingest(&mut self) -> Option<Response> {
        self.transport.ingest(self.bus.read())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
