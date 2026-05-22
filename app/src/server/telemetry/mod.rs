mod collector;
mod context;
pub mod context_provider;
mod events;
mod macros;
pub mod rudder_message;
pub mod secret_redaction;

use std::path::{Path, PathBuf};

use anyhow::Result;
pub use collector::*;
pub use context::telemetry_context;
pub use events::*;

use crate::auth::UserUid;
use crate::settings::PrivacySettingsSnapshot;

/// Filename for file where telemetry events are written on app quit.
const RUDDER_TELEMETRY_EVENTS_FILE_NAME: &str = "rudder_telemetry_events.json";

/// Filepath where the Rudder events should be written on app quit.
fn rudder_event_file_path() -> PathBuf {
    warp_core::paths::secure_state_dir()
        .unwrap_or_else(warp_core::paths::state_dir)
        .join(RUDDER_TELEMETRY_EVENTS_FILE_NAME)
}

/// Removes all telemetry events from the app telemetry event queue.
pub fn clear_event_queue() {
    let _ = warpui::telemetry::flush_events();
}

pub struct TelemetryApi {
    pub(super) client: http_client::Client,
}

impl Default for TelemetryApi {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryApi {
    pub fn new() -> Self {
        cfg_if::cfg_if! {
            if #[cfg(test)] {
                let client = http_client::Client::new_for_test();
            } else if #[cfg(target_family = "wasm")] {
                let client = http_client::Client::default();
            } else {
                use std::time::Duration;

                let client = http_client::Client::from_client_builder(
                    // We use our own http client directly instead of the Rudderstack SDK's because using
                    // our own client gives us the ability to have universal hooks for pre/post
                    // request/response logic.
                    reqwest::Client::builder()
                        // Don't allow insecure connections; they will be rejected by
                        // the server with a 403 Forbidden.
                        .https_only(true)
                        // Keep idle connections in the pool for up to 55s. AWS
                        // Application Load Balancers will drop idle connections after
                        // 60s and the default pool idle timeout is 90s; a pool idle
                        // timeout longer than the server timeout can lead to errors
                        // upon trying to use an idle connection.
                        .pool_idle_timeout(Duration::from_secs(55))
                        .connect_timeout(Duration::from_secs(10)),
                ).expect("Client should be constructed since we use a compatibility layer to use reqwest::Client");
            }
        }

        Self { client }
    }

    // Batches up telemetry events from the global queue and sends a Message to the Rudderstack API.
    // Returns the number of events that were flushed.
    pub async fn flush_events(&self, _settings_snapshot: PrivacySettingsSnapshot) -> Result<usize> {
        let events = warpui::telemetry::flush_events();
        let event_count = events.len();
        Ok(event_count)
    }

    /// Flushes events directly to Rudder that were previously written into a file at `path`
    /// (likely via a call to `write_events_to_disk`).
    pub async fn flush_persisted_events_to_rudder(
        &self,
        _path: &Path,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        // Telemetry flushing is disabled for this fork.
        Ok(())
    }

    /// Writes the last `max_event_count` events into disk. This is useful for persisting events
    /// where we can't make a network call to Rudder (such as when the app quits). To flush these
    /// events to Rudder, call `flush_events_to_rudder_from_disk`.
    pub fn flush_and_persist_events(
        &self,
        _max_event_count: usize,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        let _ = warpui::telemetry::flush_events();
        Ok(())
    }

    /// Sends a `TelemetryEvent` to the Rudderstack API.
    pub async fn send_telemetry_event(
        &self,
        _user_id: Option<UserUid>,
        _anonymous_id: String,
        _event: impl warp_core::telemetry::TelemetryEvent,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
