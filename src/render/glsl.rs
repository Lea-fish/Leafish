// Copyright 2016 Matthew Collins
// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::HashMap;

#[derive(Default)]
pub struct Registry {
    shaders: HashMap<String, String>,
    shader_version: String,
}

impl Registry {
    pub fn new(shader_version: &str) -> Registry {
        Registry {
            shaders: Default::default(),
            shader_version: shader_version.to_string(),
        }
    }

    pub fn register(&mut self, name: &str, source: &str) {
        if self.shaders.contains_key(name) {
            panic!("shader {} is already defined", name);
        }
        self.shaders
            .insert(name.to_owned(), source.trim().to_owned());
    }

    fn add_version(&self, out: &mut String) {
        out.push_str(&self.shader_version);
        out.push('\n');
        if self.shader_version.ends_with(" es") {
            out.push_str(
                r#"precision mediump float;
precision mediump sampler2DArray;
#define ES
"#,
            );
        }
    }

    pub fn get(&self, name: &str) -> String {
        let mut out = String::new();
        self.add_version(&mut out);
        self.get_internal(&mut out, name);
        out
    }

    pub fn get_define(&self, name: &str, define: &str) -> String {
        let mut out = String::new();
        self.add_version(&mut out);
        out.push_str("#define ");
        out.push_str(define);
        out.push('\n');
        self.get_internal(&mut out, name);
        out
    }

    fn get_internal(&self, out: &mut String, name: &str) {
        let src = self.shaders.get(name).unwrap();
        for line in src.lines() {
            if let Some(stripped) = line.strip_prefix("#include ") {
                let inc = stripped.trim();
                self.get_internal(out, inc);
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
    }
}
