#![no_std]
pub mod messages;

use messages::{Broadcast, Message, SlaveMessage, UpdateRequest};
use rs485_transport::{Address, AppData, DataFrame, Response, Transport, MASTER_ADDRESS};
use serde::Serialize;
use serde_cbor::{de::from_slice_with_scratch, ser::SliceWrite, Serializer};

pub struct Palantir {
    scratch_appdata: AppData,
    transport: Transport,
}

impl Palantir {
    pub fn new(device_address: Address) -> Self {
        Palantir {
            scratch_appdata: AppData::new(),
            transport: match device_address {
                MASTER_ADDRESS => Transport::new_master(),
                _ => Transport::new_slave(device_address),
            },
        }
    }

    #[inline]
    fn deserialize(&mut self, frame: DataFrame) -> Option<SlaveMessage> {
        self.scratch_appdata.clear();
        match from_slice_with_scratch::<Broadcast>(&frame.app_data(), &mut self.scratch_appdata) {
            Ok(bcast) => return Some(SlaveMessage::Broadcast(bcast)),
            _ => (),
        };

        self.scratch_appdata.clear();
        match from_slice_with_scratch::<UpdateRequest>(&frame.app_data(), &mut self.scratch_appdata)
        {
            Ok(upreq) => Some(SlaveMessage::UpdateRequest(upreq)),
            _ => None,
        }
    }

    pub fn send<M: Message + Serialize>(&mut self, message: M) -> Result<(), ()> {
        self.scratch_appdata.clear();
        let writer = SliceWrite::new(&mut self.scratch_appdata);
        let mut ser = Serializer::new(writer);
        match message.serialize(&mut ser) {
            Err(_) => return Err(()),
            _ => (),
        };

        let writer = ser.into_inner();
        let size = writer.bytes_written();
        match self.transport.send(&self.scratch_appdata[..size]) {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    pub fn poll(&mut self) -> Option<SlaveMessage> {
        let frame = self.transport.parse_data_buffer()?;
        self.deserialize(frame)
    }

    pub fn ingest(&mut self, byte: u16) -> Option<Response> {
        self.transport.ingest(byte)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
