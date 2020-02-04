use crate::transport::DataFrame;
use core::convert::{TryFrom, TryInto};
use heapless::{consts::*, Vec};

use crate::transport::AppData;

pub trait Message<'a>: TryFrom<&'a [u8]> {
    const ID: MessageID;

    fn to_payload(&self) -> Result<AppData, ()>;
}

#[repr(u8)]
pub enum MessageID {
    Broadcast = 0,
    DiscoveryRequest = 1,
    DiscoveryAck = 2,
    UpdateRequest = 3,
    SolenoidUpdate = 4,
}

impl TryFrom<u8> for MessageID {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == (MessageID::Broadcast as u8) => Ok(MessageID::Broadcast),
            x if x == (MessageID::DiscoveryRequest as u8) => Ok(MessageID::DiscoveryRequest),
            x if x == (MessageID::DiscoveryAck as u8) => Ok(MessageID::DiscoveryAck),
            x if x == (MessageID::UpdateRequest as u8) => Ok(MessageID::UpdateRequest),
            _ => Err(()),
        }
    }
}

pub struct ReceivedMessage {
    data: AppData,
}

impl ReceivedMessage {
    pub fn new(frame: DataFrame) -> Self {
        let mut data: AppData = Vec::new();
        ReceivedMessage {
            data: match data.extend_from_slice(frame.app_data()) {
                _ => data,
            },
        }
    }

    pub fn to_discovery_request(&self) -> Option<DiscoveryRequest> {
        let arr_ref: &[u8] = self.data.as_ref();
        let res: DiscoveryRequest = match arr_ref.try_into() {
            Ok(v) => v,
            _ => return None,
        };
        Some(res)
    }

    pub fn to_discovery_ack(&self) -> Option<DiscoveryAck> {
        let arr_ref: &[u8] = self.data.as_ref();
        let res: DiscoveryAck = match arr_ref.try_into() {
            Ok(v) => v,
            _ => return None,
        };
        Some(res)
    }
}

bitfield! {
    pub struct Broadcast([u8; 10]);
    impl Debug;
    u16;
}

impl<'a> Message<'a> for Broadcast {
    const ID: MessageID = MessageID::Broadcast;

    fn to_payload(&self) -> Result<AppData, ()> {
        let mut ret: AppData = Vec::new();
        ret.push(Self::ID as u8).unwrap();
        ret.extend_from_slice(&self.0)?;
        Ok(ret)
    }
}

impl<'a> TryFrom<&'a [u8]> for Broadcast {
    type Error = ();

    fn try_from(r: &'a [u8]) -> Result<Self, Self::Error> {
        if r.len() == 0 {
            return Err(());
        }

        match r[0] {
            x if x == MessageID::Broadcast as u8 => {
                let mut buf: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
                for (place, data) in buf.iter_mut().zip(r[1..].iter()) {
                    *place = *data;
                }
                Ok(Broadcast(buf))
            }
            _ => Err(()),
        }
    }
}

pub struct DiscoveryRequest;

impl<'a> Message<'a> for DiscoveryRequest {
    const ID: MessageID = MessageID::DiscoveryRequest;

    fn to_payload(&self) -> Result<AppData, ()> {
        let mut ret: AppData = Vec::new();
        ret.push(Self::ID as u8).unwrap();
        Ok(ret)
    }
}

impl<'a> TryFrom<&'a [u8]> for DiscoveryRequest {
    type Error = ();

    fn try_from(r: &'a [u8]) -> Result<Self, Self::Error> {
        if r.len() == 0 {
            return Err(());
        }

        match r[0] {
            x if x == MessageID::DiscoveryRequest as u8 => Ok(DiscoveryRequest),
            _ => Err(()),
        }
    }
}

pub struct DiscoveryAck;

impl<'a> Message<'a> for DiscoveryAck {
    const ID: MessageID = MessageID::DiscoveryAck;

    fn to_payload(&self) -> Result<AppData, ()> {
        let mut ret: AppData = Vec::new();
        ret.push(Self::ID as u8).unwrap();
        Ok(ret)
    }
}

impl<'a> TryFrom<&'a [u8]> for DiscoveryAck {
    type Error = ();

    fn try_from(r: &'a [u8]) -> Result<Self, Self::Error> {
        if r.len() == 0 {
            return Err(());
        }

        match r[0] {
            x if x == MessageID::DiscoveryAck as u8 => Ok(DiscoveryAck),
            _ => Err(()),
        }
    }
}

bitfield! {
    pub struct UpdateRequest(u32);
}

impl<'a> Message<'a> for UpdateRequest {
    const ID: MessageID = MessageID::UpdateRequest;

    fn to_payload(&self) -> Result<AppData, ()> {
        let mut ret: AppData = Vec::new();
        ret.push(Self::ID as u8).unwrap();
        ret.extend_from_slice(&self.0.to_le_bytes())?;
        Ok(ret)
    }
}

impl<'a> TryFrom<&'a [u8]> for UpdateRequest {
    type Error = ();

    fn try_from(r: &'a [u8]) -> Result<Self, Self::Error> {
        if r.len() == 0 {
            return Err(());
        }

        match r[0] {
            x if x == MessageID::UpdateRequest as u8 => {
                let mut buf: [u8; 4] = [0, 0, 0, 0];
                for (place, data) in buf.iter_mut().zip(r[1..].iter()) {
                    *place = *data;
                }
                Ok(UpdateRequest(u32::from_le_bytes(buf)))
            }
            _ => Err(()),
        }
    }
}

bitfield! {
    pub struct SolenoidUpdate(u32);
    impl Debug;
    u8;
    pub inputs, set_inputs: 11, 0;
    pub outputs, set_outputs: 18, 12;
}

impl<'a> Message<'a> for SolenoidUpdate {
    const ID: MessageID = MessageID::SolenoidUpdate;

    fn to_payload(&self) -> Result<AppData, ()> {
        let mut ret: AppData = Vec::new();
        ret.push(Self::ID as u8).unwrap();
        ret.extend_from_slice(&self.0.to_le_bytes())?;
        Ok(ret)
    }
}

impl<'a> TryFrom<&'a [u8]> for SolenoidUpdate {
    type Error = ();

    fn try_from(r: &'a [u8]) -> Result<Self, Self::Error> {
        if r.len() == 0 {
            return Err(());
        }

        match r[0] {
            x if x == MessageID::SolenoidUpdate as u8 => {
                let mut buf: [u8; 4] = [0, 0, 0, 0];
                for (place, data) in buf.iter_mut().zip(r[1..].iter()) {
                    *place = *data;
                }
                Ok(SolenoidUpdate(u32::from_le_bytes(buf)))
            }
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn update_request_space_ok() {
        let ureq = UpdateRequest(0);
        assert_eq!(ureq.to_payload().is_ok(), true);
    }

    #[test]
    fn solenoid_update_from_slice() {
        let data: MessagePayload = SolenoidUpdate(32456).to_payload().unwrap();
        let data_ref: &[u8] = data.as_ref();
        let update: SolenoidUpdate = match data_ref.try_into() {
            Ok(update) => update,
            _ => panic!("failed to parse solenoid update"),
        };
        assert_eq!(data, update.to_payload().unwrap());
    }
}
