// Copyright 2016 Matthew Collins
// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::nbt;
use crate::protocol::{self, Serializable};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io;

#[derive(Debug, Clone)]
pub struct Stack {
    pub id: isize,
    pub count: isize,
    pub damage: Option<isize>,
    pub tag: Option<nbt::NamedTag>,
}

impl Default for Stack {
    fn default() -> Stack {
        Stack {
            id: -1,
            count: 0,
            damage: None,
            tag: None,
        }
    }
}

impl Serializable for Option<Stack> {
    fn read_from<R: io::Read>(buf: &mut R) -> Result<Option<Stack>, protocol::Error> {
        let protocol_version = protocol::current_protocol_version();

        if protocol_version >= 404 {
            let present = buf.read_u8()? != 0;
            if !present {
                return Ok(None);
            }
        }

        let id = if protocol_version >= 404 {
            protocol::VarInt::read_from(buf)?.0 as isize
        } else {
            buf.read_i16::<BigEndian>()? as isize
        };

        if id == -1 {
            return Ok(None);
        }
        let count = buf.read_u8()? as isize;
        let damage = if protocol_version >= 404 {
            // 1.13.2+ stores damage in the NBT
            None
        } else {
            Some(buf.read_i16::<BigEndian>()? as isize)
        };

        let tag: Option<nbt::NamedTag> = if protocol_version >= 47 {
            Serializable::read_from(buf)?
        } else {
            // 1.7 uses a different slot data format described on https://wiki.vg/index.php?title=Slot_Data&diff=6056&oldid=4753
            let tag_size = buf.read_i16::<BigEndian>()?;
            if tag_size != -1 {
                for _ in 0..tag_size {
                    let _ = buf.read_u8()?;
                }
                // TODO: decompress zlib NBT for 1.7
                None
            } else {
                None
            }
        };

        Ok(Some(Stack {
            id: id as isize,
            count,
            damage,
            tag,
        }))
    }
    fn write_to<W: io::Write>(&self, buf: &mut W) -> Result<(), protocol::Error> {
        let protocol_version = protocol::current_protocol_version();
        if protocol_version >= 404 {
            match *self {
                Some(ref val) => {
                    buf.write_u8(1)?; //present
                    crate::protocol::VarInt(val.id as i32).write_to(buf)?;
                    buf.write_u8(val.count as u8)?;
                    val.tag.write_to(buf)?;
                }
                None => {
                    buf.write_u8(0)?; // not present
                }
            }
        } else {
            match *self {
                Some(ref val) => {
                    buf.write_i16::<BigEndian>(val.id as i16)?;
                    buf.write_u8(val.count as u8)?;
                    buf.write_i16::<BigEndian>(val.damage.unwrap_or(0) as i16)?;
                    // TODO: compress zlib NBT if 1.7
                    val.tag.write_to(buf)?;
                }
                None => buf.write_i16::<BigEndian>(-1)?,
            }
        }
        Result::Ok(())
    }
}
