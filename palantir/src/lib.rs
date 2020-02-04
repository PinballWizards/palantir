#![no_std]

#[macro_use]
extern crate bitfield;

pub mod messages;

mod parser;
mod transport;

use messages::*;
use transport::{Address, Response, Transport, MASTER_ADDRESS};

use core::convert::TryFrom;
use heapless::{consts::*, Vec};
use nom::number::complete::le_u8;

pub trait Bus {
    fn send(&mut self, data: &[u16]);
    fn read(&mut self) -> u16;
}

pub enum Error {
    NotMaster,
    SendToSelf,
    InvalidDiscoveryAck,
    Other,
}

pub type SlaveAddresses = Vec<Address, U7>;

pub struct Palantir<B: Bus> {
    transport: Transport,
    address: Address,
    bus: B,
}

impl<B: Bus> Palantir<B> {
    pub fn new(device_address: Address, bus: B) -> Self {
        Palantir {
            transport: match device_address {
                MASTER_ADDRESS => Transport::new_master(),
                _ => Transport::new_slave(device_address),
            },
            address: device_address,
            bus: bus,
        }
    }

    fn wait_for_discovery_ack(&mut self, address: Address) -> Result<(), Error> {
        let msg = loop {
            match self.poll() {
                Some(m) => break m,
                _ => (),
            }
        };

        match msg {
            ReceivedMessage::DiscoveryAck(addr, _) => {
                if addr == address {
                    Ok(())
                } else {
                    Err(Error::InvalidDiscoveryAck)
                }
            }
            _ => Err(Error::InvalidDiscoveryAck),
        }
    }

    pub fn discover_devices(&mut self, addresses: &SlaveAddresses) -> Result<(), Error> {
        for address in addresses.iter() {
            self.send(*address, DiscoveryAck)?;
            self.wait_for_discovery_ack(*address)?;
        }
        Ok(())
    }

    pub fn send<M: Message>(&mut self, address: Address, message: M) -> Result<(), Error> {
        if address == self.address {
            return Err(Error::SendToSelf);
        }

        let payload = match message.to_payload() {
            Ok(v) => v,
            Err(_) => return Err(Error::Other),
        };
        self.bus.send(&match self.transport.send(address, payload) {
            Ok(data) => data,
            _ => return Err(Error::Other),
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
                Ok(msg) => Some(ReceivedMessage::Broadcast(frame.address(), msg)),
                _ => None,
            },
            MessageID::DiscoveryRequest => match DiscoveryRequest::try_from(i) {
                Ok(msg) => Some(ReceivedMessage::DiscoveryRequest(frame.address(), msg)),
                _ => None,
            },
            MessageID::DiscoveryAck => match DiscoveryAck::try_from(i) {
                Ok(msg) => Some(ReceivedMessage::DiscoveryAck(frame.address(), msg)),
                _ => None,
            },
            MessageID::UpdateRequest => match UpdateRequest::try_from(i) {
                Ok(msg) => Some(ReceivedMessage::UpdateRequest(frame.address(), msg)),
                _ => None,
            },
            MessageID::SolenoidUpdate => match SolenoidUpdate::try_from(i) {
                Ok(msg) => Some(ReceivedMessage::SolenoidUpdate(frame.address(), msg)),
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
