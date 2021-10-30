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
