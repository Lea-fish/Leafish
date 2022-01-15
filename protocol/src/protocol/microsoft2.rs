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

use crate::protocol::login::{Account, AccountImpl, AccountType};
use anyhow::Context;
use rand::distributions::Alphanumeric;
use rand::Rng;
use reqwest::Url;
use serde::Deserialize;
use serde_json::json;
use sha1::{self, Digest};
use std::borrow::Cow;
use std::env;
use std::str::FromStr;
use std::sync::mpsc;
use warp::Filter;

const JOIN_URL: &str = "https://sessionserver.mojang.com/session/minecraft/join";

// CREDIT: Big parts of this implementation were taken from: https://github.com/ALinuxPerson/mcsoft-auth

pub struct MicrosoftAccount {}

impl AccountImpl for MicrosoftAccount {
    fn login(&self, name: &str, password: &str, token: &str) -> Result<Account, super::Error> {
        resolve_account_data(String::new(), String::new(), todo!(), None, name.to_string(), password.to_string());
    }

    fn refresh(&self, account: Account, token: &str) -> Result<Account, super::Error> {
        todo!()
    }

    fn join_server(
        &self,
        account: &Account,
        server_id: &str,
        shared_key: &[u8],
        public_key: &[u8],
    ) -> Result<(), super::Error> {
        // FIXME: Does this work?
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
            "-".to_owned() + &hash_val[..]
        } else {
            hash_val.to_owned()
        };

        let join_msg = json!({
            "accessToken": account.verification_tokens.get(2).unwrap(),
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

#[derive(Deserialize)]
pub struct Query {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize)]
pub struct CompleteToken {
    pub expires_in: u64,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct AccessToken {
    pub access_token: String,
}

#[derive(Deserialize)]
pub struct Xui {
    #[serde(rename = "uhs")]
    pub user_hash: String,
}

#[derive(Deserialize)]
pub struct DisplayClaims {
    pub xui: Vec<Xui>,
}

#[derive(Deserialize)]
pub struct AuthenticateWithXboxLiveOrXsts {
    #[serde(rename = "Token")]
    pub token: String,

    #[serde(rename = "DisplayClaims")]
    pub display_claims: DisplayClaims,
}

#[derive(Deserialize, PartialEq)]
pub struct Item {
    pub name: Cow<'static, str>,
    // pub signature: String, // todo: signature verification
}

impl Item {
    pub const PRODUCT_MINECRAFT: Self = Self {
        name: Cow::Borrowed("product_minecraft"),
    };
    pub const GAME_MINECRAFT: Self = Self {
        name: Cow::Borrowed("game_minecraft"),
    };
}

#[derive(Deserialize)]
pub struct Store {
    pub items: Vec<Item>,

    // pub signature: String, // todo: signature verification
    #[serde(rename = "keyId")]
    pub key_id: String,
}

impl AuthenticateWithXboxLiveOrXsts {
    pub fn extract_essential_information(self) -> anyhow::Result<(String, String)> {
        let token = self.token;
        let user_hash = self
            .display_claims
            .xui
            .into_iter()
            .next()
            .context("no xui found")?
            .user_hash;

        Ok((token, user_hash))
    }
}

#[derive(Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
}

fn receive_query(port: u16) -> Query {
    let (sender, receiver) = mpsc::sync_channel(1);
    let route = warp::get()
        .and(warp::filters::query::query())
        .map(move |query: Query| {
            sender.send(query).expect("failed to send query");
            "Successfully received query"
        });

    tokio::task::spawn(warp::serve(route).run(([127, 0, 0, 1], port)));

    receiver.recv().expect("channel has hung up")
}

fn random_string() -> String {
    rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

fn resolve_account_data(client_id: String, client_secret: String, redirect_uri: Url, port: Option<u16>, name: String, password: String) -> anyhow::Result<Account> {
    match redirect_uri.domain() {
        Some(domain) => anyhow::ensure!(
            domain == "localhost" || domain == "127.0.0.1",
            "domain '{}' isn't valid, it must be '127.0.0.1' or 'localhost'",
            domain
        ),
        None => anyhow::bail!("the redirect uri must have a domain"),
    }

    let port = port.unwrap_or(80);
    let state = random_string();
    let url = format!(
        "https://login.live.com/oauth20_authorize.srf\
?client_id={}\
&response_type=code\
&redirect_uri={}\
&scope=XboxLive.signin%20offline_access\
&state={}",
        client_id, redirect_uri, state
    );

    if let Err(error) = webbrowser::open(&url) {
        println!("error opening browser: {}", error);
        println!("use this link instead:\n{}", url)
    }

    // Now awaiting code
    let query = receive_query(port);

    anyhow::ensure!(
        query.state == state,
        "state mismatch: got state '{}' from query, but expected state was '{}'",
        query.state,
        state
    );

    let client = reqwest::blocking::Client::new();

    // Now getting the access token
    let complete_token: CompleteToken = client
        .post("https://login.live.com/oauth20_token.srf")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", query.code),
            ("grant_type", "authorization_code".to_string()),
            ("redirect_uri", redirect_uri.to_string()),
        ])
        .send()?
        .json()?;
    let json = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={}", complete_token.access_token),
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    // Now authenticating with Xbox Live
    let auth_with_xbl: AuthenticateWithXboxLiveOrXsts = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&json)
        .send()?
        .json()?;
    let (token, user_hash) = auth_with_xbl.extract_essential_information()?;

    // Now getting an Xbox Live Security Token (XSTS)
    let json = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });
    let auth_with_xsts: AuthenticateWithXboxLiveOrXsts = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&json)
        .send()?
        .json()?;
    let (token, _) = auth_with_xsts.extract_essential_information()?;
    // Now authenticating with Minecraft
    let access_token: AccessToken = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({
            "identityToken": format!("XBL3.0 x={};{}", user_hash, token)
        }))
        .send()?
        .json()?;
    let access_token = access_token.access_token;

    // Checking for game ownership

    // FIXME: i don't know how to do signature verification, so we just have to assume the signatures are
    // FIXME: valid :)
    let store: Store = client
        .get("https://api.minecraftservices.com/entitlements/mcstore")
        .bearer_auth(&access_token)
        .send()?
        .json()?;

    anyhow::ensure!(
        store.items.contains(&Item::PRODUCT_MINECRAFT),
        "product_minecraft item doesn't exist. do you really own the game?"
    );

    anyhow::ensure!(
        store.items.contains(&Item::GAME_MINECRAFT),
        "game_minecraft item doesn't exist. do you really own the game?"
    );

    // Getting game profile

    let profile: Profile = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&access_token)
        .send()?
        .json()?;

    let mut account = Account::new(profile.name, Some(profile.id), AccountType::Microsoft);
    account.verification_tokens.push(name);
    account.verification_tokens.push("".to_string());
    account.verification_tokens.push(access_token);
    account.verification_tokens.push(complete_token.refresh_token);

    Ok(account)
}
