#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate bitfield;

#[cfg(feature = "feather_bus")]
pub mod feather_bus;
pub mod messages;

mod parser;
mod transport;

use messages::*;
use transport::{Address, Response, Transport, MASTER_ADDRESS};

use core::convert::TryFrom;
use heapless::{consts::*, Vec};
use nb;
use nom::number::complete::le_u8;

pub type SlaveAddresses = Vec<Address, U7>;

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
    transport: Transport,
    address: Address,
    bus: B,
    slaves: SlaveAddresses,
}

impl<B: Bus> Palantir<B> {
    pub fn new_slave(device_address: Address, bus: B) -> Self {
        Palantir {
            transport: Transport::new_slave(device_address),
            address: device_address,
            bus: bus,
            slaves: Vec::new(),
        }
    }

    pub fn new_master(slaves: SlaveAddresses, bus: B) -> Self {
        Palantir {
            transport: Transport::new_master(),
            address: MASTER_ADDRESS,
            bus: bus,
            slaves: slaves,
        }
    }

    fn wait_for_discovery_ack(&mut self, address: Address) -> Result<(), Error> {
        let msg = loop {
            self.ingest();
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

    fn wait_for_discovery_request(&mut self) -> Result<(), Error> {
        let msg = loop {
            self.ingest();
            match self.poll() {
                Some(m) => break m,
                _ => (),
            }
        };

        match msg {
            ReceivedMessage::DiscoveryRequest(_, _) => self.send(MASTER_ADDRESS, DiscoveryAck),
            _ => Err(Error::InvalidDiscoveryReq),
        }
    }

    /// This should only be called by the master device at startup!
    pub fn discover_devices(&mut self) -> Result<(), Error> {
        for address in self.slaves.clone().iter() {
            self.send(*address, DiscoveryAck)?;
            self.wait_for_discovery_ack(*address)?;
        }
        Ok(())
    }

    /// This should be called only by slave devices at startup.
    pub fn discovery_mode(&mut self) -> Result<(), Error> {
        self.wait_for_discovery_request()
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

    pub fn ingest(&mut self) -> Option<Response> {
        match self.bus.read() {
            Ok(v) => self.transport.ingest(v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::spsc::Queue;

    struct MockBus {
        buf: Queue<u16, U260>,
    }

    impl MockBus {
        fn new() -> Self {
            Self { buf: Queue::new() }
        }
    }

    impl Bus for MockBus {
        type Error = ();
        fn send(&mut self, data: &[u16]) {
            for byte in data.iter() {
                let _ = self.buf.enqueue(*byte);
            }
        }

        fn read(&mut self) -> nb::Result<u16, Self::Error> {
            match self.buf.dequeue() {
                Some(v) => Ok(v),
                None => Err(nb::Error::Other(())),
            }
        }
    }

    fn get_mocked_slave(address: Address) -> Palantir<MockBus> {
        Palantir::new_slave(address, MockBus::new())
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
    fn discovery_ack_transmit() {
        let msg = DiscoveryAck;
        let slave_addr: Address = 0x2;
        let mut palantir = get_mocked_slave(slave_addr);

        palantir.send(slave_addr + 1, msg);

        for _ in 0..260 {
            palantir.ingest();
        }

        let recv_msg = match palantir.poll() {
            Some(msg) => msg,
            None => panic!("did not get sent message"),
        };

        match recv_msg {
            ReceivedMessage::DiscoveryRequest(_, _) => (),
            _ => panic!("did not get discovery request"),
        };
    }
}
