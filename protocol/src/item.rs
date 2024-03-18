// Copyright 2016 Matthew Collins
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::format::Component;
use crate::nbt::{self, NamedTag, Tag};
use crate::protocol::{self, Serializable};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::io;

#[derive(Debug, Clone)]
pub struct Stack {
    pub id: isize,
    pub count: isize,
    pub damage: Option<isize>,
    pub meta: ItemMeta,
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            id: -1,
            count: 0,
            damage: None,
            meta: ItemMeta(None),
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
            meta: ItemMeta(tag),
        }))
    }
    fn write_to<W: io::Write>(&self, buf: &mut W) -> Result<(), protocol::Error> {
        let protocol_version = protocol::current_protocol_version();
        if protocol_version >= 404 {
            match *self {
                Some(ref val) => {
                    buf.write_u8(1)?; // present
                    crate::protocol::VarInt(val.id as i32).write_to(buf)?;
                    buf.write_u8(val.count as u8)?;
                    val.meta.0.write_to(buf)?;
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
                    val.meta.0.write_to(buf)?;
                }
                None => buf.write_i16::<BigEndian>(-1)?,
            }
        }
        Result::Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ItemMeta(Option<NamedTag>);

impl ItemMeta {
    fn display(&self) -> Option<&HashMap<String, Tag>> {
        match self.0.as_ref() {
            Some(tag) => tag
                .1
                .as_compound()
                .and_then(|comp| comp.get("display").map(|val| val.as_compound()))
                .flatten(),
            None => None,
        }
    }

    pub fn display_name(&self) -> Option<Component> {
        self.display()
            .and_then(|val| {
                val.get("Name")
                    .map(|name| name.as_str().map(Component::from_str))
            })
            .flatten()
    }

    pub fn lore(&self) -> Vec<Component> {
        self.display()
            .and_then(|val| {
                val.get("Lore").map(|lore| {
                    lore.as_list().map(|lore| {
                        lore.iter()
                            .filter_map(|line| line.as_str().map(Component::from_str))
                            .collect::<Vec<_>>()
                    })
                })
            })
            .flatten()
            .unwrap_or_default()
    }

    pub fn repair_cost(&self) -> Option<i32> {
        match self.0.as_ref() {
            Some(tag) => tag
                .1
                .as_compound()
                .and_then(|comp| comp.get("RepairCost").map(|val| val.as_int()))
                .flatten(),
            None => None,
        }
    }

    pub fn enchantments(&self) -> Vec<Enchantment> {
        match self.0.as_ref() {
            Some(tag) => tag
                .1
                .as_compound()
                .and_then(|comp| {
                    comp.get("ench").map(|ench| {
                        ench.as_list().map(|enchs| {
                            enchs
                                .iter()
                                .filter_map(|ench| {
                                    ench.as_compound().and_then(|ench| {
                                        ench.get("lvl")
                                            .and_then(|lvl| lvl.as_short())
                                            .zip(ench.get("id").and_then(|id| id.as_short()))
                                            .and_then(|(level, id)| {
                                                Enchantment::new(id as u16, level)
                                            })
                                    })
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                })
                .flatten()
                .unwrap_or(vec![]),
            None => vec![],
        }
    }
}

pub struct Enchantment {
    pub ty: EnchantmentTy,
    pub level: i16,
}

impl Enchantment {
    #[inline]
    fn new(id: u16, level: i16) -> Option<Self> {
        Some(Self {
            ty: EnchantmentTy::from_id(id)?,
            level,
        })
    }
}

#[repr(u32)]
pub enum EnchantmentTy {
    Protection = 0,
    FireProtection = 1,
    FeatherFalling = 2,
    BlastProtection = 3,
    ProjectileProtection = 4,
    Respiration = 5,
    AquaAffinity = 6,
    Thorns = 7,
    DepthStrider = 8,
    Sharpness = 16,
    Smite = 17,
    BaneOfArthropods = 18,
    Knockback = 19,
    FireAspect = 20,
    Looting = 21,
    Efficiency = 32,
    SilkTouch = 33,
    Unbreaking = 34,
    Fortune = 35,
    Power = 48,
    Punch = 49,
    Flame = 50,
    Infinity = 51,
    LuckOfTheSea = 61,
    Lure = 62,
}

impl EnchantmentTy {
    fn from_id(id: u16) -> Option<Self> {
        match id {
            0 => Some(Self::Protection),
            1 => Some(Self::FireProtection),
            2 => Some(Self::FeatherFalling),
            3 => Some(Self::BlastProtection),
            4 => Some(Self::ProjectileProtection),
            5 => Some(Self::Respiration),
            6 => Some(Self::AquaAffinity),
            7 => Some(Self::Thorns),
            8 => Some(Self::DepthStrider),
            16 => Some(Self::Sharpness),
            17 => Some(Self::Smite),
            18 => Some(Self::BaneOfArthropods),
            19 => Some(Self::Knockback),
            20 => Some(Self::FireAspect),
            21 => Some(Self::Looting),
            32 => Some(Self::Efficiency),
            33 => Some(Self::SilkTouch),
            34 => Some(Self::Fortune),
            35 => Some(Self::Fortune),
            48 => Some(Self::Power),
            49 => Some(Self::Punch),
            50 => Some(Self::Flame),
            51 => Some(Self::Infinity),
            61 => Some(Self::LuckOfTheSea),
            62 => Some(Self::Lure),
            _ => None,
        }
    }
}
