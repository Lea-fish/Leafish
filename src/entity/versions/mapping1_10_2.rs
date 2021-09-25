use crate::entity::EntityType;

pub fn to_id(entity_type: EntityType) -> i16 {
    crate::entity::versions::mapping1_12_2::to_id(entity_type)
}

pub fn to_entity_type(type_id: i16) -> EntityType {
    crate::entity::versions::mapping1_12_2::to_entity_type(type_id)
}
