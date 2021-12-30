// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::direction::Direction;
use bevy_ecs::prelude::*;
use std::fmt;
use std::ops;

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Position {
    pub fn new(x: i32, y: i32, z: i32) -> Position {
        Position { x, y, z }
    }

    pub fn shift(self, dir: Direction) -> Position {
        let (ox, oy, oz) = dir.get_offset();
        self + (ox, oy, oz)
    }

    pub fn shift_by(self, dir: Direction, by: i32) -> Position {
        let (ox, oy, oz) = dir.get_offset();
        self + (ox * by, oy * by, oz * by)
    }
}

impl ops::Add<Position> for Position {
    type Output = Position;

    fn add(self, o: Position) -> Position {
        Position {
            x: self.x + o.x,
            y: self.y + o.y,
            z: self.z + o.z,
        }
    }
}

impl ops::Add<(i32, i32, i32)> for Position {
    type Output = Position;

    fn add(self, (x, y, z): (i32, i32, i32)) -> Position {
        Position {
            x: self.x + x,
            y: self.y + y,
            z: self.z + z,
        }
    }
}

impl ops::Sub<Position> for Position {
    type Output = Position;

    fn sub(self, o: Position) -> Position {
        Position {
            x: self.x - o.x,
            y: self.y - o.y,
            z: self.z - o.z,
        }
    }
}

impl ops::Sub<(i32, i32, i32)> for Position {
    type Output = Position;

    fn sub(self, (x, y, z): (i32, i32, i32)) -> Position {
        Position {
            x: self.x - x,
            y: self.y - y,
            z: self.z - z,
        }
    }
}

impl Default for Position {
    fn default() -> Position {
        Position::new(0, 0, 0)
    }
}

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{},{},{}>", self.x, self.y, self.z)
    }
}
