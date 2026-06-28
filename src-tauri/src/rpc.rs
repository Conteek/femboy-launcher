use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Rpc {
    client: Arc<Mutex<Option<DiscordIpcClient>>>,
    pending_activity: Arc<Mutex<Option<(String, String)>>>,
    connecting: Arc<AtomicBool>,
}

impl Rpc {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
            pending_activity: Arc::new(Mutex::new(Some((
                "В лаунчере".to_string(),
                "Просматривает главную страницу".to_string(),
            )))),
            connecting: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn connect(&self) {
        // Prevent multiple simultaneous connection attempts
        if self.connecting.swap(true, Ordering::SeqCst) {
            return;
        }

        let client_arc = self.client.clone();
        let pending_arc = self.pending_activity.clone();
        let connecting_arc = self.connecting.clone();

        std::thread::spawn(move || {
            match DiscordIpcClient::new("1515101378811396389") {
                Ok(mut c) => {
                    // Try to connect multiple times if needed, or just once?
                    // Let's stick to once for now to not block the thread forever,
                    // but set_activity will trigger it again if needed.
                    if c.connect().is_ok() {
                        if let Ok(pending) = pending_arc.lock() {
                            if let Some((details, state)) = pending.as_ref() {
                                let start = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs() as i64;

                                let activity = activity::Activity::new()
                                    .details(details)
                                    .state(state)
                                    .assets(
                                        activity::Assets::new()
                                            .large_image("logo")
                                            .large_text("Femboy Launcher"),
                                    )
                                    .timestamps(activity::Timestamps::new().start(start));

                                let _ = c.set_activity(activity);
                            }
                        }
                        *client_arc.lock().unwrap() = Some(c);
                    }
                }
                Err(_) => {}
            }
            connecting_arc.store(false, Ordering::SeqCst);
        });
    }

    pub fn set_activity(&self, details: &str, state: &str) {
        // Store as pending
        if let Ok(mut pending) = self.pending_activity.lock() {
            *pending = Some((details.to_string(), state.to_string()));
        }

        let start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let activity = activity::Activity::new()
            .details(details)
            .state(state)
            .assets(
                activity::Assets::new()
                    .large_image("logo")
                    .large_text("Femboy Launcher"),
            )
            .timestamps(activity::Timestamps::new().start(start));

        if let Ok(mut guard) = self.client.lock() {
            let mut needs_connect = false;
            if let Some(c) = guard.as_mut() {
                if c.set_activity(activity).is_err() {
                    // Connection lost, clear client so it can be reconnected
                    *guard = None;
                    needs_connect = true;
                }
            } else {
                needs_connect = true;
            }

            if needs_connect {
                drop(guard); // Release lock before calling connect which might spawn a thread that wants it
                self.connect();
            }
        }
    }

    pub fn clear(&self) {
        if let Ok(mut pending) = self.pending_activity.lock() {
            *pending = None;
        }
        if let Ok(mut guard) = self.client.lock() {
            if let Some(c) = guard.as_mut() {
                let _ = c.clear_activity();
            }
        }
    }
}
