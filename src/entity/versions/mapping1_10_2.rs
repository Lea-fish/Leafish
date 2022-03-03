// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::entity::EntityType;

pub fn to_id(entity_type: EntityType) -> i16 {
    crate::entity::versions::mapping1_12_2::to_id(entity_type)
}

pub fn to_entity_type(type_id: i16) -> EntityType {
    crate::entity::versions::mapping1_12_2::to_entity_type(type_id)
}
