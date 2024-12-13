use crate::chat_messages::ChatMessage;
use gauth::app::Auth;
use log::{debug, warn};
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LiveBroadcastsResponse {
    items: Vec<Broadcast>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Broadcast {
    snippet: BroadcastSnippet,
    status: Status,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BroadcastSnippet {
    live_chat_id: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Status {
    life_cycle_status: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MessageListResponse {
    polling_interval_millis: u64,
    next_page_token: String,
    items: Vec<MessageItem>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MessageItem {
    snippet: MessageSnippet,
    author_details: AuthorDetails,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MessageSnippet {
    author_channel_id: String,
    display_message: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AuthorDetails {
    display_name: String,
}
pub(crate) async fn listen_to_youtube_chat<F>(on_message: F)
where
    F: Fn(ChatMessage) + Send + Sync,
{
    let token = get_auth_token().await.unwrap();
    let youtube_client = YouTubeClient::new(token.clone());
    let broadcasts = youtube_client.get_my_live_broadcasts().await.unwrap();

    assert!(!broadcasts.items.is_empty());
    let this_broadcast = &broadcasts.items[0];
    assert_eq!(this_broadcast.status.life_cycle_status, "live");

    let mut page_token = String::new();
    let mut sleep_duration;
    loop {
        debug!("getting youtube messages...");
        let response = youtube_client
            .get_message_list(&this_broadcast.snippet.live_chat_id, &page_token)
            .await;

        match response {
            Ok(response) => {
                page_token = response.next_page_token;

                let item_count = response.items.len();
                debug!("got {} youtube messages", { item_count });
                for item in response.items {
                    on_message(item.into());
                }

                sleep_duration = if item_count == 0 {
                    Duration::from_secs(3)
                } else {
                    Duration::from_millis(response.polling_interval_millis)
                };
            }
            Err(e) => {
                warn!("GetMessageList failed with: {:?}", e);
                sleep_duration = Duration::from_secs(10);
            }
        }

        sleep(sleep_duration).await;
    }
}

impl From<MessageItem> for ChatMessage {
    fn from(value: MessageItem) -> Self {
        Self {
            author_channel_id: value.snippet.author_channel_id,
            author_name: Some(value.author_details.display_name),
            message: value.snippet.display_message,
        }
    }
}

async fn get_auth_token() -> anyhow::Result<String> {
    let auth_client = Auth::from_file(
        "youtube_credentials.json",
        vec!["https://www.googleapis.com/auth/youtube.readonly"],
    )?;
    let token = auth_client.access_token().await?;
    Ok(token)
}

struct YouTubeClient {
    auth_token: String,
    http_client: reqwest::Client,
}

impl YouTubeClient {
    fn new(auth_token: String) -> Self {
        Self {
            auth_token,
            http_client: reqwest::ClientBuilder::new().build().unwrap(),
        }
    }

    async fn get_my_live_broadcasts(&self) -> anyhow::Result<LiveBroadcastsResponse> {
        let response = self.http_client
        .get("https://www.googleapis.com/youtube/v3/liveBroadcasts?part=snippet,status&mine=true")
        .header(AUTHORIZATION, self.auth_token.clone())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
        Ok(response)
    }

    async fn get_message_list(
        &self,
        live_chat_id: impl AsRef<str>,
        page_token: impl AsRef<str>,
    ) -> anyhow::Result<MessageListResponse> {
        let response=  self.http_client.get(format!(
            "https://www.googleapis.com/youtube/v3/liveChat/messages?liveChatId={}&part=snippet,authorDetails&pageToken={}",
            live_chat_id.as_ref(),
            page_token.as_ref()
        ))
        .header(AUTHORIZATION, self.auth_token.clone())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
        Ok(response)
    }
}
