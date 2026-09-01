//! Async API v4 quick start.

use blooio::BlooioCreds;
use blooio::v4::Client;
use blooio::v4::resources::messages::SendMessage;
use blooio::v4::types::{AppClipFields, MessageBadge, MessageContentFields, Recipient};

#[tokio::main]
async fn main() -> blooio::Result<()> {
    let client = Client::from_env()?;
    let creds = BlooioCreds::from_env()?;
    let account = client.account(&creds);

    let _me = account.me().get().await?;
    let _message = account
        .messages()
        .send(SendMessage::new(
            Recipient::identifier("+15551234567"),
            MessageContentFields::text("hello from v4").badge(MessageBadge::SentWithSiri),
        ))
        .await?;

    let _app_clip = MessageContentFields::app_clip(
        AppClipFields::bundle_id("com.example.app.Clip").title("Example"),
    );
    Ok(())
}
