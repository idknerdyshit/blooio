//! Blocking API v4 quick start.

use blooio::BlooioCreds;
use blooio::v4::BlockingClient;

fn main() -> blooio::Result<()> {
    let client = BlockingClient::from_env()?;
    let creds = BlooioCreds::from_env()?;
    let _me = client.account(&creds).me().get()?;
    Ok(())
}
