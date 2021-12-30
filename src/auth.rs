// Copyright 2016 Matthew Collins
// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::console;
use std::marker::PhantomData;

pub const AUTH_CLIENT_TOKEN: console::CVar<String> = console::CVar {
    ty: PhantomData,
    name: "auth_client_token",
    description: r#"auth_client_token is a token that stays static between sessions.
Used to identify this client vs others."#,
    mutable: false,
    serializable: true,
    default: &|| String::new(),
};

pub fn register_vars(vars: &mut console::Vars) {
    vars.register(AUTH_CLIENT_TOKEN);
}
