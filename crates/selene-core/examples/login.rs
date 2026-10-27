//! Signs in with a device code and prints the Minecraft profile.
//!
//! ```sh
//! cargo run -p selene-core --example login
//! ```

use selene_core::auth::{self, DeviceLogin};
use selene_core::http;

#[tokio::main]
async fn main() -> selene_core::Result<()> {
    let client = http::client()?;
    let code = DeviceLogin::request(&client).await?;
    println!(
        "Open {} and enter {}",
        code.verification_uri, code.user_code
    );

    let tokens = code.finish(&client).await?;
    println!("Microsoft sign-in done");

    let session = auth::minecraft_login(&client, &tokens.access_token).await?;
    println!(
        "Signed in as {} ({})",
        session.profile.name, session.profile.id
    );
    Ok(())
}
