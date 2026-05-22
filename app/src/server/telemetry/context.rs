//! Module that builds the static context attached to telemetry-shaped payloads.
//! This fork keeps the context empty so no environment metadata is emitted.

use std::sync::OnceLock;

use serde_json::Value;

use super::rudder_message::Message as RudderMessage;

static TELEMETRY_CONTEXT: OnceLock<TelemetryContext> = OnceLock::new();

/// Newtype representing a [`Value`] with a serialized version of the context that we send to
/// Rudderstack.
/// See https://www.rudderstack.com/docs/event-spec/standard-events/common-fields/#contextual-fields.
pub struct TelemetryContext(Value);

impl TelemetryContext {
    pub fn as_value(&self) -> Value {
        self.0.clone()
    }
}

impl TelemetryContext {
    fn new() -> Self {
        Self(Value::Object(Default::default()))
    }
}

/// Extension trait used to attach a telemetry context.
pub(super) trait AttachContext {
    /// Attaches a context to the given object.
    fn attach_context(&mut self);
}

impl AttachContext for RudderMessage {
    /// Attaches the context to the [`RudderMessage`]. Note this is currently last write wins; if a
    /// message already has a `context` set it will be overridden.
    // TODO(alokedesai): Merge the incoming context with the static `TelemetryContext`, if set.
    fn attach_context(&mut self) {
        let context = telemetry_context().as_value();
        match self {
            RudderMessage::Identify(identify) => {
                identify.context = Some(context);
            }
            RudderMessage::Track(track) => track.context = Some(context),
            RudderMessage::Page(page) => page.context = Some(context),
            RudderMessage::Screen(screen) => screen.context = Some(context),
            RudderMessage::Group(group) => group.context = Some(context),
            RudderMessage::Alias(alias) => alias.context = Some(context),
            RudderMessage::Batch(batch) => batch.context = Some(context),
        }
    }
}

/// Returns the telemetry context
/// that should be attached to all telemetry events associated to this client.
///
/// [Rudderstack](https://www.rudderstack.com/docs/event-spec/standard-events/common-fields/#contextual-fields)
pub fn telemetry_context() -> &'static TelemetryContext {
    TELEMETRY_CONTEXT.get_or_init(TelemetryContext::new)
}
