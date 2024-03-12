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

use crate::protocol::login::{Account, AccountImpl};
use serde_json::json;
use sha1::Digest;

const JOIN_URL: &str = "https://sessionserver.mojang.com/session/minecraft/join";

pub struct MicrosoftAccount {}

impl AccountImpl for MicrosoftAccount {
    fn login(&self, _name: &str, _password: &str, _token: &str) -> Result<Account, super::Error> {
        unimplemented!()
    }

    fn refresh(&self, account: Account, _token: &str) -> Result<Account, super::Error> {
        Ok(account)
    }

    fn join_server(
        &self,
        account: &Account,
        server_id: &str,
        shared_key: &[u8],
        public_key: &[u8],
    ) -> Result<(), super::Error> {
        let mut hasher = sha1::Sha1::new();
        hasher.update(server_id.as_bytes());
        hasher.update(shared_key);
        hasher.update(public_key);
        let mut hash = hasher.finalize();

        // Mojang uses a hex method which allows for
        // negatives so we have to account for that.
        let negative = (hash[0] & 0x80) == 0x80;
        if negative {
            twos_compliment(&mut hash);
        }
        let hash_str = hash
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .join("");
        let hash_val = hash_str.trim_start_matches('0');
        let hash_str = if negative {
            "-".to_owned() + hash_val
        } else {
            hash_val.to_owned()
        };

        let join_msg = json!({
            "accessToken": account.verification_tokens.get(2).unwrap(), // FIXME: make this the only verification_token!
            "selectedProfile": account.uuid.as_ref().unwrap(),
            "serverId": hash_str
        });
        let join = serde_json::to_string(&join_msg).unwrap();

        let client = reqwest::blocking::Client::new();
        let res = client
            .post(JOIN_URL)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(join)
            .send()?;

        if res.status() == reqwest::StatusCode::NO_CONTENT {
            Ok(())
        } else {
            Err(super::Error::Err("Failed to auth with server".to_owned()))
        }
    }

    fn append_head_img_data(&self, _account: &mut Account) -> Result<(), super::Error> {
        Ok(())
    }
}

fn twos_compliment(data: &mut [u8]) {
    let mut carry = true;
    for i in (0..data.len()).rev() {
        data[i] = !data[i];
        if carry {
            carry = data[i] == 0xFF;
            data[i] = data[i].wrapping_add(1);
        }
    }
}
