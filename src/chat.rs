use rsnano_core::Account;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChatMessage {
    pub author_channel_id: String,
    pub author_name: Option<String>,
    pub message: String,
}

impl ChatMessage {
    pub fn new_test_instance() -> Self {
        Self {
            author_name: Some("John Doe".to_owned()),
            author_channel_id: "abc".to_owned(),
            message: "test message".to_owned(),
        }
    }

    pub fn new_test_instance_for_account(account: Account) -> Self {
        Self {
            author_name: Some("John Doe".to_owned()),
            author_channel_id: "abc".to_owned(),
            message: account.encode_account(),
        }
    }
}
