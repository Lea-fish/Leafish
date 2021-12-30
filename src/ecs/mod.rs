// Copyright 2016 Matthew Collins
// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use bevy_ecs::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;

// System labels to enforce a run order of our systems
#[derive(SystemLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SystemExecStage {
    PreClearRemoveHandling, // TODO: This is a mess, clean it up as soon as bevy fixed the various remove detection issues!
    PreNormal,
    Normal,
    Render,
    RemoveHandling,
}

#[derive(Default)]
pub struct Manager {
    pub world: World,
    pub schedule: Arc<RwLock<Schedule>>,
    pub entity_schedule: Arc<RwLock<Schedule>>,
}
