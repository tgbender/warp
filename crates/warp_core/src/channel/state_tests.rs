use super::{ChannelState, derive_http_origin_from_ws_url};

#[test]
fn default_oss_state_has_no_remote_service_endpoints() {
    let state = ChannelState::init();

    assert!(state.config.server_config.server_root_url.is_empty());
    assert!(state.config.server_config.rtc_server_url.is_empty());
    assert!(
        state
            .config
            .server_config
            .session_sharing_server_url
            .is_none()
    );
    assert!(state.config.server_config.firebase_auth_api_key.is_empty());
    assert!(state.config.oz_config.oz_root_url.is_empty());
    assert!(state.config.telemetry_config.is_none());
    assert!(state.config.crash_reporting_config.is_none());
    assert!(state.config.autoupdate_config.is_none());
    assert!(state.config.mcp_static_config.is_none());
}

#[test]
fn wss_becomes_https_and_strips_path() {
    let got = derive_http_origin_from_ws_url("wss://rtc.app.warp.dev/graphql/v2");
    assert_eq!(got.as_deref(), Some("https://rtc.app.warp.dev"));
}

#[test]
fn ws_becomes_http_and_preserves_port() {
    let got = derive_http_origin_from_ws_url("ws://localhost:8080/graphql/v2");
    assert_eq!(got.as_deref(), Some("http://localhost:8080"));
}

#[test]
fn unparseable_input_returns_none() {
    assert!(derive_http_origin_from_ws_url("not a url").is_none());
    assert!(derive_http_origin_from_ws_url("https://app.warp.dev").is_none());
}
