use crate::entity::EntityType;
use leafish_protocol::protocol::Version;

mod mapping1_15_2;
mod mapping1_16_5;
mod mapping1_14_4;

pub fn to_id(entity_type: EntityType, version: Version) -> i16 {
    match version {
        Version::V1_7 => 0,
        Version::V1_8 => 0,
        Version::V1_9 => 0,
        Version::V1_10 => 0,
        Version::V1_11 => 0,
        Version::V1_12 => 0,
        Version::V1_13 => 0,
        Version::V1_14 => mapping1_14_4::to_id(entity_type),
        Version::V1_15 => mapping1_15_2::to_id(entity_type),
        Version::V1_16 => mapping1_16_5::to_id(entity_type),
        _ => 0,
    }
}

pub fn to_entity_type(id: i16, version: Version) -> EntityType {
    match version {
        Version::V1_7 => EntityType::Unknown,
        Version::V1_8 => EntityType::Unknown,
        Version::V1_9 => EntityType::Unknown,
        Version::V1_10 => EntityType::Unknown,
        Version::V1_11 => EntityType::Unknown,
        Version::V1_12 => EntityType::Unknown,
        Version::V1_13 => EntityType::Unknown,
        Version::V1_14 => mapping1_14_4::to_entity_type(id),
        Version::V1_15 => mapping1_15_2::to_entity_type(id),
        Version::V1_16 => mapping1_16_5::to_entity_type(id),
        _ => EntityType::Unknown,
    }
}