// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::entity::EntityType;
use leafish_protocol::protocol::Version;

mod mapping1_10_2;
mod mapping1_11_2;
mod mapping1_12_2;
mod mapping1_13_2;
mod mapping1_14_4;
mod mapping1_15_2;
mod mapping1_16_5;
mod mapping1_7_10;
mod mapping1_8_8;
mod mapping1_9_4;

pub fn to_id(entity_type: EntityType, version: Version) -> i16 {
    match version {
        Version::V1_7 => mapping1_7_10::to_id(entity_type),
        Version::V1_8 => mapping1_8_8::to_id(entity_type),
        Version::V1_9 => mapping1_9_4::to_id(entity_type),
        Version::V1_10 => mapping1_10_2::to_id(entity_type),
        Version::V1_11 => mapping1_11_2::to_id(entity_type),
        Version::V1_12 => mapping1_12_2::to_id(entity_type),
        Version::V1_13 => mapping1_13_2::to_id(entity_type),
        Version::V1_14 => mapping1_14_4::to_id(entity_type),
        Version::V1_15 => mapping1_15_2::to_id(entity_type),
        Version::V1_16 => mapping1_16_5::to_id(entity_type),
        _ => -1,
    }
}

pub fn to_entity_type(id: i16, version: Version) -> EntityType {
    match version {
        Version::V1_7 => mapping1_7_10::to_entity_type(id),
        Version::V1_8 => mapping1_8_8::to_entity_type(id),
        Version::V1_9 => mapping1_9_4::to_entity_type(id),
        Version::V1_10 => mapping1_10_2::to_entity_type(id),
        Version::V1_11 => mapping1_11_2::to_entity_type(id),
        Version::V1_12 => mapping1_12_2::to_entity_type(id),
        Version::V1_13 => mapping1_13_2::to_entity_type(id),
        Version::V1_14 => mapping1_14_4::to_entity_type(id),
        Version::V1_15 => mapping1_15_2::to_entity_type(id),
        Version::V1_16 => mapping1_16_5::to_entity_type(id),
        _ => EntityType::Unknown,
    }
}
