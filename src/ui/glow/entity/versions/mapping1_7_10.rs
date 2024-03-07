use crate::ui::glow::entity::EntityType;

pub fn to_id(entity_type: EntityType) -> i16 {
    super::mapping1_12_2::to_id(entity_type)
}

pub fn to_entity_type(type_id: i16) -> EntityType {
    super::mapping1_12_2::to_entity_type(type_id)
}
