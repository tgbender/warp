use super::*;

#[test]
fn telemetry_record_and_persist_are_noops() {
    let telemetry_api = TelemetryApi::new();

    warpui::telemetry::record_event(
        Some("user".into()),
        "anonymous_id".to_owned(),
        "event name".into(),
        None,
        false,
        warpui::time::get_current_time(),
    );
    telemetry_api
        .flush_and_persist_events(10, PrivacySettingsSnapshot::mock())
        .expect("telemetry persistence no-op should succeed");

    assert_eq!(warpui::telemetry::flush_events().len(), 0);
}
