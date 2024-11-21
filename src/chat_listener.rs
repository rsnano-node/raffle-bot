use crate::chat::ChatMessage;
use log::info;
use std::time::Duration;
use tokio::{spawn, time::interval};
use youtube_chat::{item::MessageItem, live_chat::LiveChatClientBuilder};

pub(crate) async fn listen_to_chat<F>(stream_url: String, on_message: F)
where
    F: Fn(ChatMessage) + Send + Sync + 'static,
{
    info!("stream url: '{}'", stream_url);
    let mut client = LiveChatClientBuilder::new()
        .url(stream_url)
        .unwrap()
        .on_chat(move |item| {
            for m in item.message {
                if let MessageItem::Text(msg) = m {
                    on_message(ChatMessage {
                        author_channel_id: item.author.channel_id.clone(),
                        author_name: item.author.name.clone(),
                        message: msg,
                    })
                }
            }
        })
        .build();

    client.start().await.unwrap();
    let forever = spawn(async move {
        let mut interval = interval(Duration::from_millis(3000));
        loop {
            interval.tick().await;
            client.execute().await;
        }
    });

    forever.await.unwrap();
}
