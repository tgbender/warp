mod event_store;

use std::borrow::Cow;

use chrono::{DateTime, Utc};
use event_store::*;
pub use event_store::{Event, EventPayload};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde_json::Value;

lazy_static! {
    static ref TELEMETRY: Mutex<EventStore> = Mutex::new(EventStore::new());
}

#[macro_export]
macro_rules! record_telemetry_from_ctx {
    ($user_id: expr, $anonymous_id: expr, $name:expr, $payload: expr, $contains_ugc: expr, $ctx: expr) => {{
        let timestamp = $crate::time::get_current_time();
        $ctx.background_executor()
            .spawn(async move {
                $crate::telemetry::record_event(
                    $user_id,
                    $anonymous_id,
                    $name,
                    $payload,
                    $contains_ugc,
                    timestamp,
                )
            })
            .detach();
    }};
}

#[macro_export]
macro_rules! record_telemetry_on_executor {
    ($user_id: expr, $anonymous_id: expr, $name:expr, $payload: expr, $contains_ugc: expr, $executor: expr) => {{
        let timestamp = $crate::time::get_current_time();
        let _ = $executor
            .spawn(async move {
                $crate::telemetry::record_event(
                    $user_id,
                    $anonymous_id,
                    $name,
                    $payload,
                    $contains_ugc,
                    timestamp,
                )
            })
            .detach();
    }};
}

/// Creates a new `Event`, but does not record it. It is up to the caller to determine when, and
/// how, the event should be recorded.
pub fn create_event(
    user_id: Option<String>,
    anonymous_id: String,
    name: Cow<'static, str>,
    payload: Option<Value>,
    contains_ugc: bool,
    timestamp: DateTime<Utc>,
) -> Event {
    let mut telemetry = TELEMETRY.lock();
    telemetry.create_event(
        user_id,
        anonymous_id,
        name,
        payload,
        contains_ugc,
        timestamp,
    )
}

pub fn record_event(
    _user_id: Option<String>,
    _anonymous_id: String,
    _name: Cow<'static, str>,
    _payload: Option<Value>,
    _contains_ugc: bool,
    _timestamp: DateTime<Utc>,
) {
    // Telemetry recording is disabled for this fork.
}

pub fn record_identify_user_event(
    _user_id: String,
    _anonymous_id: String,
    _timestamp: DateTime<Utc>,
) {
    // Telemetry recording is disabled for this fork.
}

/// Adds a 'App Active' event to the global event queue.  This should only be called in an async
/// context.
pub fn record_app_active_event(
    _user_id: Option<String>,
    _anonymous_id: String,
    _timestamp: DateTime<Utc>,
) {
    // Telemetry recording is disabled for this fork.
}

pub fn flush_events() -> Vec<Event> {
    TELEMETRY.lock().events.drain(..).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_recording_entry_points_are_noops() {
        let timestamp = Utc::now();
        let _ = flush_events();

        record_event(
            Some("user".to_owned()),
            "anonymous".to_owned(),
            "test event".into(),
            None,
            false,
            timestamp,
        );
        record_identify_user_event("user".to_owned(), "anonymous".to_owned(), timestamp);
        record_app_active_event(Some("user".to_owned()), "anonymous".to_owned(), timestamp);

        assert!(flush_events().is_empty());
    }
}
