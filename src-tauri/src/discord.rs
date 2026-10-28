//! "Playing Selene Launcher" status in Discord.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use discord_rich_presence::activity::{Activity, Assets, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use serde::Deserialize;

/// The "Selene Launcher" application in the Discord developer portal; its name is what the
/// status shows. The id is public.
const CLIENT_ID: &str = "1564950619213799584";
/// How often to try again while Discord is closed.
const RETRY_EVERY: Duration = Duration::from_secs(20);

/// What the status should say. The UI sends the text, already translated.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Presence {
    pub details: String,
    pub state: Option<String>,
    /// Unix seconds; Discord counts the time played from it.
    pub started_at: Option<i64>,
}

pub struct Discord {
    sender: Sender<Option<Presence>>,
}

impl Discord {
    pub fn start() -> Self {
        let (sender, receiver) = mpsc::channel::<Option<Presence>>();
        std::thread::spawn(move || {
            let mut client: Option<DiscordIpcClient> = None;
            let mut wanted: Option<Presence> = None;
            let mut shown = false;
            loop {
                match receiver.recv_timeout(RETRY_EVERY) {
                    Ok(presence) => {
                        wanted = presence;
                        shown = false;
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                if shown {
                    continue;
                }
                if client.is_none() {
                    let mut fresh = DiscordIpcClient::new(CLIENT_ID);
                    if fresh.connect().is_err() {
                        continue;
                    }
                    client = Some(fresh);
                }
                let Some(connected) = client.as_mut() else {
                    continue;
                };
                let sent = match &wanted {
                    Some(presence) => connected.set_activity(activity(presence)),
                    None => connected.clear_activity(),
                };
                // Discord was closed in between; reconnect on the next round.
                shown = sent.is_ok();
                if !shown {
                    client = None;
                }
            }
            if let Some(mut connected) = client {
                let _ = connected.close();
            }
        });
        Self { sender }
    }

    /// `None` clears the status.
    pub fn set(&self, presence: Option<Presence>) {
        let _ = self.sender.send(presence);
    }
}

fn activity(presence: &Presence) -> Activity<'_> {
    let mut activity = Activity::new().details(presence.details.as_str()).assets(
        Assets::new()
            .large_image("logo")
            .large_text("Selene Launcher"),
    );
    if let Some(state) = &presence.state {
        activity = activity.state(state.as_str());
    }
    if let Some(started_at) = presence.started_at {
        activity = activity.timestamps(Timestamps::new().start(started_at));
    }
    activity
}
