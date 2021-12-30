// Copyright 2016 Matthew Collins
// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

mod metadata;
pub use self::metadata::*;

pub mod bit;
pub mod hash;
pub mod nibble;

use bevy_ecs::prelude::*;

#[derive(Component, Clone, Copy, Debug)]
pub enum GameMode {
    NotSet = -1,
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
}

impl GameMode {
    pub fn from_int(val: i32) -> GameMode {
        match val {
            3 => GameMode::Spectator,
            2 => GameMode::Adventure,
            1 => GameMode::Creative,
            0 => GameMode::Survival,
            -1 => GameMode::NotSet,
            _ => GameMode::Adventure,
        }
    }

    pub fn can_fly(&self) -> bool {
        matches!(*self, GameMode::Creative | GameMode::Spectator)
    }

    pub fn always_fly(&self) -> bool {
        matches!(*self, GameMode::Spectator)
    }

    pub fn noclip(&self) -> bool {
        matches!(*self, GameMode::Spectator)
    }

    pub fn can_interact_with_world(&self) -> bool {
        matches!(
            *self,
            GameMode::Creative | GameMode::Survival | GameMode::NotSet
        )
    }
}
