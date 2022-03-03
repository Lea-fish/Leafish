// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::protocol::packet::play::serverbound::PluginMessageServerbound;
use crate::protocol::packet::play::serverbound::PluginMessageServerbound_i16;
use crate::protocol::{Serializable, VarShort};
use leafish_protocol::protocol::Conn;

pub struct Brand {
    pub brand: String,
}

impl Brand {
    pub fn write_to(self, conn: &mut Conn) {
        let protocol_version = crate::protocol::current_protocol_version();

        let mut data = vec![];
        Serializable::write_to(&self.brand, &mut data).unwrap();
        if protocol_version >= 47 {
            let channel_name = if protocol_version >= 404 {
                "minecraft:brand"
            } else {
                "MC|Brand"
            };
            let packet = PluginMessageServerbound {
                channel: channel_name.into(),
                data,
            };
            conn.write_packet(packet).unwrap();
        } else {
            let packet = PluginMessageServerbound_i16 {
                channel: "MC|Brand".into(),
                data: crate::protocol::LenPrefixedBytes::<VarShort>::new(data),
            };
            conn.write_packet(packet).unwrap();
        }
    }
}
