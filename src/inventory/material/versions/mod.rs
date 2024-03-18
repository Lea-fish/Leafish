use crate::inventory::Material;
use shared::Version;

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

// FIXME: support tags!
pub fn to_material(
    id: u16,
    damage: Option<isize>,
    tag: Option<String>,
    version: Version,
) -> Material {
    match version {
        Version::V1_7 => mapping1_7_10::to_material(id, damage.unwrap()),
        Version::V1_8 => mapping1_8_8::to_material(id, damage.unwrap()),
        Version::V1_9 => mapping1_9_4::to_material(id, damage.unwrap()),
        Version::V1_10 => mapping1_10_2::to_material(id, damage.unwrap()),
        Version::V1_11 => mapping1_11_2::to_material(id, damage.unwrap()),
        Version::V1_12 => mapping1_12_2::to_material(id, damage.unwrap()),
        Version::V1_13 => mapping1_13_2::to_material(id),
        Version::V1_13_2 => mapping1_13_2::to_material(id),
        Version::V1_14 => mapping1_14_4::to_material(id),
        Version::V1_15 => mapping1_15_2::to_material(id),
        Version::V1_16 => mapping1_16_5::to_material(id),
        Version::V1_16_2 => mapping1_16_5::to_material(id),
        _ => Material::Air,
    }
}

#[allow(dead_code)]
pub fn to_id(material: Material, version: Version) -> u16 {
    match version {
        Version::V1_7 => mapping1_7_10::to_id(material),
        Version::V1_8 => mapping1_8_8::to_id(material),
        Version::V1_9 => mapping1_9_4::to_id(material),
        Version::V1_10 => mapping1_10_2::to_id(material),
        Version::V1_11 => mapping1_11_2::to_id(material),
        Version::V1_12 => mapping1_12_2::to_id(material),
        Version::V1_13 => mapping1_13_2::to_id(material),
        Version::V1_13_2 => mapping1_13_2::to_id(material),
        Version::V1_14 => mapping1_14_4::to_id(material),
        Version::V1_15 => mapping1_15_2::to_id(material),
        Version::V1_16 => mapping1_16_5::to_id(material),
        Version::V1_16_2 => mapping1_16_5::to_id(material),
        _ => 0,
    }
}

pub fn get_stack_size(material: Material, version: Version) -> u8 {
    match version {
        Version::V1_7 => mapping1_7_10::get_stack_size(material),
        Version::V1_8 => mapping1_8_8::get_stack_size(material),
        Version::V1_9 => mapping1_9_4::get_stack_size(material),
        Version::V1_10 => mapping1_10_2::get_stack_size(material),
        Version::V1_11 => mapping1_11_2::get_stack_size(material),
        Version::V1_12 => mapping1_12_2::get_stack_size(material),
        Version::V1_13 => mapping1_13_2::get_stack_size(material),
        Version::V1_13_2 => mapping1_13_2::get_stack_size(material),
        Version::V1_14 => mapping1_14_4::get_stack_size(material),
        Version::V1_15 => mapping1_15_2::get_stack_size(material),
        Version::V1_16 => mapping1_16_5::get_stack_size(material),
        Version::V1_16_2 => mapping1_16_5::get_stack_size(material),
        _ => 64,
    }
}
