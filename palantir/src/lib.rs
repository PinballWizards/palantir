#![no_std]

#[macro_use]
extern crate bitfield;

pub mod messages;

use palantir_transport::{Address, Response, Transport, MASTER_ADDRESS};

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

    pub fn send<'a, M: messages::Message<'a>>(&mut self, message: M) -> Result<(), ()> {
        let payload = message.to_payload()?;
        self.bus.send(&match self.transport.send(&payload) {
            Ok(data) => data,
            _ => return Err(()),
        });
        Ok(())
    }

    pub fn poll(&mut self) -> Option<messages::ReceivedMessage> {
        let frame = self.transport.parse_data_buffer()?;
        Some(messages::ReceivedMessage::new(frame))
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
