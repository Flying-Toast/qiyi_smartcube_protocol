use crate::crc::crc16;
use crate::cubestate::CubeState;
use anyhow::{bail, Result};
use btleplug::api::BDAddr;
use thiserror::Error;

#[derive(Debug)]
#[repr(u16)]
enum Opcode {
    CubeHello = 0x2,
    StateChange = 0x3,
    SyncConfirmation = 0x4,
}

impl Opcode {
    fn from_u16(x: u16) -> Result<Self> {
        if x == Self::CubeHello as u16 {
            Ok(Self::CubeHello)
        } else if x == Self::StateChange as u16 {
            Ok(Self::StateChange)
        } else if x == Self::SyncConfirmation as u16 {
            Ok(Self::SyncConfirmation)
        } else {
            bail!(ParseError::BadOpcode { bad_opcode: x })
        }
    }
}

/// A cube->app message.
#[derive(Debug)]
pub struct C2aMessage<'a> {
    // /// Reference to bytes 3-7 for use in ACKs
    ack_head: &'a [u8],
    body: C2aBody<'a>,
}

impl<'a> C2aMessage<'a> {
    fn needs_ack(&self) -> bool {
        match self.body {
            C2aBody::CubeHello(_) => true,
            // TODO: not every state change needs an ack???
            C2aBody::StateChange(_) => true,
        }
    }

    /// Returns `Some(ack)` if this message needs to be ACKed;
    /// returns `None` if it doesn't need an ACK.
    // TODO: make structs for app->cube messages instead of returning &[u8] here
    pub fn make_ack(&self) -> Option<&'a [u8]> {
        if self.needs_ack() {
            Some(self.ack_head)
        } else {
            None
        }
    }

    pub fn body(&self) -> &C2aBody<'a> {
        &self.body
    }
}

/// The "body" of a cube->app message is the decrypted contents
/// minus the `0xfe` prefix, length, opcode, padding, and checksum.
#[derive(Debug)]
pub enum C2aBody<'a> {
    CubeHello(CubeHello<'a>),
    StateChange(StateChange<'a>),
}

#[derive(Debug)]
pub struct CubeHello<'a> {
    pub state: CubeState<'a>,
}

#[derive(Debug)]
pub struct StateChange<'a> {
    pub state: CubeState<'a>,
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Missing magic `0xfe` byte at start of message")]
    BadMagic,
    #[error("Tried to get a byte that wasn't in the message??")]
    OutOfRange,
    #[error("Expected message to be longer")]
    TooShort,
    #[error("Invalid checksum")]
    FailedChecksum,
    #[error("Invalid opcode (got {bad_opcode}")]
    BadOpcode { bad_opcode: u16 },
}

struct Parser<'a> {
    bytes_from_start: &'a [u8],
    cursor: usize,
}

impl<'a> Parser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes_from_start: bytes,
            cursor: 0,
        }
    }

    fn get_bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.cursor + n > self.bytes_from_start.len() {
            bail!(ParseError::OutOfRange)
        } else {
            let ret = &self.bytes_from_start[self.cursor..self.cursor + n];
            self.cursor += n;
            Ok(ret)
        }
    }

    fn peek_checksum_bytes(&self) -> &'a [u8] {
        &self.bytes_from_start
    }

    fn get_bytes_from_end_for_checksum(&mut self, n: usize) -> Result<&'a [u8]> {
        if n > self.remaining_len() {
            bail!(ParseError::OutOfRange)
        } else {
            let ret = &self.bytes_from_start[self.bytes_from_start.len() - n..];
            self.bytes_from_start = &self.bytes_from_start[..self.bytes_from_start.len() - n];
            Ok(ret)
        }
    }

    fn remaining_len(&self) -> usize {
        self.bytes_from_start.len() - self.cursor
    }

    fn trim_padding(&mut self, length_after: u8) {
        self.bytes_from_start = &self.bytes_from_start[..length_after as usize];
    }

    fn get_u8(&mut self) -> Result<u8> {
        Ok(self.get_bytes(1)?[0])
    }

    fn get_u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.get_bytes(2)?.try_into().unwrap()))
    }

    fn get_u16_from_end(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.get_bytes_from_end_for_checksum(2)?.try_into().unwrap(),
        ))
    }
}

pub fn make_app_hello(mac: BDAddr) -> Vec<u8> {
    // fill the 11-byte unknown field with zeros
    let mut v = vec![0; 11];

    let mut mac = mac.into_inner();
    mac.reverse();

    v.extend_from_slice(&mac);

    v
}

/// Given the bytes of an **decrypted** message, parse them into a cube->app message.
pub fn parse_c2a_message(bytes: &[u8]) -> Result<C2aMessage> {
    let mut p = Parser::new(bytes);

    if p.get_u8()? != 0xfe {
        bail!(ParseError::BadMagic);
    }

    let length = p.get_u8()?;
    if p.remaining_len() < length as usize {
        bail!(ParseError::TooShort);
    }
    p.trim_padding(length);
    let checksum = p.get_u16_from_end()?;
    if crc16(p.peek_checksum_bytes()) != checksum {
        bail!(ParseError::FailedChecksum);
    }

    let opcode = Opcode::from_u16(p.get_u16()?)?;
    let body = match opcode {
        Opcode::CubeHello => {
            let _unknown1 = p.get_bytes(3)?;
            let rawstate = p.get_bytes(27)?;
            let _unknown2 = p.get_bytes(2)?;

            C2aBody::CubeHello(CubeHello {
                state: CubeState::from_raw(rawstate),
            })
        }
        Opcode::StateChange => {
            let _unknown1 = p.get_bytes(3)?;
            let rawstate = p.get_bytes(27)?;
            let _unknown2 = p.get_bytes(2)?;
            let _unknown3 = p.get_bytes(56)?;

            C2aBody::StateChange(StateChange {
                state: CubeState::from_raw(rawstate),
            })
        }
        Opcode::SyncConfirmation => {
            todo!()
        }
    };

    assert!(p.remaining_len() == 0);

    assert!(p.bytes_from_start.len() >= 7);
    let ack_head = &p.bytes_from_start[2..=6];

    Ok(C2aMessage { ack_head, body })
}
