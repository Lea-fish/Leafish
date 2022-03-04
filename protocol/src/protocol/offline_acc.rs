use crate::protocol::login::{Account, AccountImpl, AccountType};
use rand::{distributions::Alphanumeric, Rng};
pub struct OfflineAccount {}

impl AccountImpl for OfflineAccount {
    fn login(&self, name: &str, _password: &str, _token: &str) -> Result<Account, super::Error> {
        let test = std::iter::repeat(())
            .map(|()| rand::thread_rng().sample(&Alphanumeric) as char)
            .take(20)
            .collect();

        Ok(Account {
            name: name.to_string(),
            uuid: Some(test),
            verification_tokens: vec![name.to_string(), "".to_string(), "".to_string()],
            head_img_data: None,
            account_type: AccountType::None,
        })
    }

    fn refresh(&self, account: Account, _token: &str) -> Result<Account, super::Error> {
        Ok(account)
    }

    fn join_server(
        &self,
        _account: &Account,
        _server_id: &str,
        _shared_key: &[u8],
        _public_key: &[u8],
    ) -> Result<(), super::Error> {
        Ok(())
    }

    fn append_head_img_data(&self, _account: &mut Account) -> Result<(), super::Error> {
        Ok(())
    }
}
