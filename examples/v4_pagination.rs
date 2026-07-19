//! Cursor pagination with API v4.

use blooio::BlooioCreds;
use blooio::v4::Client;

#[tokio::main]
async fn main() -> blooio::Result<()> {
    let client = Client::from_env()?;
    let creds = BlooioCreds::from_env()?;
    let events = client
        .account(&creds)
        .events()
        .list_all()
        .collect_all()
        .await?;
    let _count = events.len();
    Ok(())
}
