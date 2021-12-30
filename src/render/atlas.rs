// Copyright 2016 Matthew Collins
// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub struct Atlas {
    free_space: Vec<Rect>,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Atlas {
    pub fn new(width: usize, height: usize) -> Atlas {
        let mut a = Atlas {
            free_space: Vec::new(),
        };
        a.free_space.push(Rect {
            x: 0,
            y: 0,
            width,
            height,
        });
        a
    }

    pub fn add(&mut self, width: usize, height: usize) -> Option<Rect> {
        let mut priority = usize::MAX;
        let mut target: Option<Rect> = None;
        let mut target_index = 0;
        // Search through and find the best fit for this texture
        for (index, free) in self.free_space.iter().enumerate() {
            if free.width >= width && free.height >= height {
                let current_priority = (free.width - width) * (free.height - height);
                if target.is_none() || current_priority < priority {
                    target = Some(*free);
                    priority = current_priority;
                    target_index = index;
                }
                // Perfect match, we can break early
                if priority == 0 {
                    break;
                }
            }
        }
        target?;
        let mut t = target.unwrap();
        let ret = Rect {
            x: t.x,
            y: t.y,
            width,
            height,
        };

        if width == t.width {
            t.y += height;
            t.height -= height;
            if t.height == 0 {
                // Remove empty sections
                self.free_space.remove(target_index);
            } else {
                self.free_space[target_index] = t;
            }
        } else {
            if t.height > height {
                // Split by height
                self.free_space.insert(
                    0,
                    Rect {
                        x: t.x,
                        y: t.y + height,
                        width,
                        height: t.height - height,
                    },
                );
                target_index += 1;
            }
            t.x += width;
            t.width -= width;
            self.free_space[target_index] = t;
        }

        Some(ret)
    }
}
