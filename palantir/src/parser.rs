use core::cell::Cell;

use crate::common::*;
use crate::messages::{message_from_data, Message};

enum ReceiverState {
    Idle,
    Receiving,
    Completed,
    Error,
}

type ReceiverError = ();

struct Receiver {
    state: ReceiverState,
    buffer: [u8; 64],
    data_length: u8,
    received: u8,
}

impl Receiver {
    pub fn new() -> Self {
        Receiver {
            state: ReceiverState::Idle,
            buffer: [0; MAX_MESSAGE_LEN],
            data_length: 0,
            received: 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.state == ReceiverState::Completed
    }

    fn reset(&mut self) {
        self.data_length = 0;
        self.received = 0;
    }

    /// This function should be called any time an Address byte is received.
    pub fn start(&mut self) {
        self.reset();
        self.state = ReceiverState::Receiving;
    }

    pub fn add_to_buffer(&mut self, data: u8) -> Result<(), ReceiverError> {
        match self.state {
            ReceiverState::Receiving => {
                if self.data_length == 0 {
                    if data as usize > MAX_MESSAGE_LEN {
                        return Err(ReceiverError);
                    }
                    self.data_length = data;
                } else if self.received < self.data_length {
                    self.buffer[self.received] = data;
                    self.received += 1;
                } else {
                    return Err(ReceiverError);
                }
            }
            ReceiverState::Idle | ReceiverState::Completed | ReceiverState::Error => {
                return Err(ReceiverError);
            }
        };

        if self.received == self.data_length {
            self.state = ReceiverState::Completed;
        }

        Ok(())
    }

    pub fn data(&self) -> &[u8] {
        self.buffer[..self.length]
    }
}

pub struct Parser {
    address: Address,
    completed: Cell<Option<Message>>,
    receiver: Receiver,
}

impl Parser {
    pub fn new(address: Address) -> Self {
        Parser {
            address,
            completed: Cell::new(None),
            receiver: Receiver::new(),
        }
    }

    fn is_address_byte(address_data: u16) -> Option<Address> {
        if address_data & (1 << 8) == 1 {
            Some(address_data as Address)
        }
        None
    }

    pub fn ingest(&mut self, data: u16) {
        let address = self.is_address_byte(data);
        if address.is_some() && address.unwrap() == self.address {
            self.receiver.start();
        } else {
            self.receiver.add_to_buffer(data as u8);
        }

        if self.receiver.is_complete() {
            match message_from_data(self.receiver.data()) {
                Ok(msg) => self.completed.set(Some(msg)),
                _ => (),
            }
        }
    }

    pub fn poll_message(&mut self) -> Option<Message> {
        self.completed.replace(None)
    }
}
