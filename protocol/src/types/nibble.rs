// Copyright 2015 Matthew Collins
// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[derive(Clone)]
pub struct Array {
    pub data: Vec<u8>,
}

impl Array {
    pub fn new(size: usize) -> Self {
        Array {
            data: vec![0; (size + 1) >> 1],
        }
    }

    pub fn new_def(size: usize, def: u8) -> Self {
        let def = (def & 0xF) | ((def & 0xF) << 4);
        Array {
            data: vec![def; (size + 1) >> 1],
        }
    }

    pub fn get(&self, idx: usize) -> u8 {
        let val = self.data[idx >> 1];
        if idx & 1 == 0 {
            val & 0xF
        } else {
            val >> 4
        }
    }

    pub fn set(&mut self, idx: usize, val: u8) {
        let i = idx >> 1;
        let old = self.data[i];
        if idx & 1 == 0 {
            self.data[i] = (old & 0xF0) | (val & 0xF);
        } else {
            self.data[i] = (old & 0x0F) | ((val & 0xF) << 4);
        }
    }
}
