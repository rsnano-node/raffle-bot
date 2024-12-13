use log::{debug, info};
use twitch_irc::{
    login::StaticLoginCredentials, message::ServerMessage, ClientConfig, SecureTCPTransport,
    TwitchIRCClient,
};

use crate::chat_messages::ChatMessage;

pub(crate) async fn listen_to_twitch_chat<F>(on_message: F)
where
    F: Fn(ChatMessage) + Send + Sync,
{
    info!("Connecting to Twitch chat...");
    // default configuration is to join chat as anonymous.
    let config = ClientConfig::default();
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

    // join a channel
    // This function only returns an error if the passed channel login name is malformed,
    // so in this simple case where the channel name is hardcoded we can ignore the potential
    // error with `unwrap`.
    client.join("gschauwecker".to_owned()).unwrap();

    info!("Twitch chat connected!");
    while let Some(message) = incoming_messages.recv().await {
        if let ServerMessage::Privmsg(msg) = message {
            debug!("Received message from twitch");
            on_message(ChatMessage {
                author_channel_id: format!("twitch-{}", msg.sender.name),
                author_name: Some(msg.sender.name),
                message: msg.message_text,
            })
        }
    }
}
