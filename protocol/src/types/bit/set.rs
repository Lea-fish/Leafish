// Copyright 2016 Matthew Collins
// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[derive(Clone, Debug)]
pub struct Set {
    data: Vec<u64>,
}

#[test]
fn test_set() {
    let mut set = Set::new(200);
    for i in 0..200 {
        if i % 3 == 0 {
            set.set(i, true)
        }
    }
    for i in 0..200 {
        if set.get(i) != (i % 3 == 0) {
            panic!("Fail")
        }
    }
}

impl Set {
    pub fn new(size: usize) -> Set {
        Set {
            data: vec![0; (size + 63) / 64],
        }
    }

    pub fn resize(&mut self, new_size: usize) {
        self.data.resize((new_size + 63) / 64, 0);
    }

    pub fn capacity(&self) -> usize {
        self.data.len() * 64
    }

    pub fn set(&mut self, i: usize, v: bool) {
        if v {
            self.data[i >> 6] |= 1 << (i & 0x3F)
        } else {
            self.data[i >> 6] &= !(1 << (i & 0x3F))
        }
    }

    pub fn get(&self, i: usize) -> bool {
        (self.data[i >> 6] & (1 << (i & 0x3F))) != 0
    }

    pub fn includes_set(&self, other: &Set) -> bool {
        for (a, b) in self.data.iter().zip(&other.data) {
            if a & b != *b {
                return false;
            }
        }
        true
    }

    pub fn or(&mut self, other: &Set) {
        for (a, b) in self.data.iter_mut().zip(&other.data) {
            *a |= *b;
        }
    }
}
