// Copyright 2026 Tine Zata
// SPDX-License-Identifier: MPL-2.0

mod test_ipc_dispatch {
    use std::sync::{Arc, Mutex};

    use mycela::app::AppState;
    use mycela::channel::ChannelContext;
    use mycela::config::AppConfig;
    use mycela::ipc::{IpcCommand, IpcErrorCode, IpcMessageKind, IpcRequest};
    use mycela::ipc_dispatch::dispatch_request;

    fn make_app_state() -> AppState {
        let config = Arc::new(AppConfig {
            title: "test".to_string(),
            home_screen: None,
            screens: Vec::new(),
        });

        #[cfg(feature = "epics")]
        let epics_ctx = Arc::new(Mutex::new(
            mycela::pvxs_sys::Context::from_env().expect("pvxs context required"),
        ));

        #[cfg(feature = "modbus")]
        let modbus_pool = mycela::modbus_client::ModbusPool::new();

        #[cfg(all(feature = "epics", feature = "modbus"))]
        let channel_ctx = ChannelContext::new(epics_ctx, modbus_pool);

        #[cfg(all(feature = "epics", not(feature = "modbus")))]
        let channel_ctx = ChannelContext::new(epics_ctx);

        #[cfg(all(not(feature = "epics"), feature = "modbus"))]
        let channel_ctx = ChannelContext::new(modbus_pool);

        #[cfg(all(not(feature = "epics"), not(feature = "modbus")))]
        let channel_ctx = ChannelContext::new();

        AppState {
            #[cfg(feature = "epics")]
            pv_server: Arc::new(Mutex::new(None)),
            config,
            channel_ctx,
            modbus_task: Arc::new(Mutex::new(None)),
            #[cfg(feature = "epics")]
            epics_start_hook: None,
            #[cfg(feature = "modbus")]
            modbus_start_hook: None,
            loopback_token: None,
        }
    }

    fn make_request(cmd: IpcCommand) -> IpcRequest {
        IpcRequest {
            v: 1,
            kind: IpcMessageKind::Request,
            id: "req-1".to_string(),
            cmd,
            token: None,
            payload: serde_json::json!({}),
            ts: 0,
        }
    }

    #[tokio::test]
    async fn test_rejects_non_request_message_kind() {
        let state = make_app_state();
        let mut request = make_request(IpcCommand::AppPing);
        request.kind = IpcMessageKind::Event;

        let response = dispatch_request(&state, request, None).await;

        assert!(!response.ok);
        assert_eq!(
            response.error.expect("error present").code,
            IpcErrorCode::PayloadInvalid
        );
    }

    #[tokio::test]
    async fn test_rejects_mutating_command_with_invalid_token() {
        let state = make_app_state();
        let mut request = make_request(IpcCommand::AppWidgetWrite);
        request.payload = serde_json::json!({ "widget_id": "x", "value": "1" });

        let response = dispatch_request(&state, request, Some("expected-token")).await;

        assert!(!response.ok);
        assert_eq!(
            response.error.expect("error present").code,
            IpcErrorCode::AuthInvalidToken
        );
    }

    #[tokio::test]
    async fn test_ping_returns_ok_response() {
        let state = make_app_state();
        let request = make_request(IpcCommand::AppPing);

        let response = dispatch_request(&state, request, None).await;

        assert!(response.ok);
        assert_eq!(response.kind, IpcMessageKind::Response);
        assert_eq!(response.result.expect("result present")["pong"], true);
    }

    #[tokio::test]
    async fn test_protocol_subscribe_commands_are_orchestrated_outside_dispatcher() {
        let state = make_app_state();
        let request = make_request(IpcCommand::EpicsPvSubscribe);

        let response = dispatch_request(&state, request, None).await;

        assert!(!response.ok);
        assert_eq!(
            response.error.expect("error present").code,
            IpcErrorCode::CmdUnknown
        );
    }
}
