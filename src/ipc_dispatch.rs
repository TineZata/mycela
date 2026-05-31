use crate::app::AppState;
use crate::ipc::{
    IpcCommand, IpcError, IpcErrorCode, IpcRequest, IpcResponse,
    IpcMessageKind,
};
use crate::protocol_control::{self, ProtocolControlError};
use serde_json::json;

pub async fn dispatch_request(
    state: &AppState,
    request: IpcRequest,
    expected_token: Option<&str>,
) -> IpcResponse {
    if request.kind != IpcMessageKind::Request {
        return error_response(
            &request.id,
            IpcErrorCode::PayloadInvalid,
            "IPC message kind must be request",
        );
    }

    if request.cmd.is_mutating() {
        match (expected_token, request.token.as_deref()) {
            (Some(expected), Some(token)) if expected == token => {}
            (Some(_), _) => {
                return error_response(
                    &request.id,
                    IpcErrorCode::AuthInvalidToken,
                    "Token missing or invalid",
                );
            }
            (None, _) => {}
        }
    }

    match request.cmd {
        IpcCommand::EpicsServerStart => match protocol_control::start_epics_runtime(state).await {
            Ok(()) => ok_response(&request.id, json!({ "running": true })),
            Err(error) => protocol_error_response(&request.id, error),
        },
        IpcCommand::EpicsServerStop => match protocol_control::stop_epics_server(state).await {
            Ok(()) => ok_response(&request.id, json!({ "running": false })),
            Err(error) => protocol_error_response(&request.id, error),
        },
        IpcCommand::EpicsServerStatusGet => ok_response(
            &request.id,
            json!({ "running": state.is_server_running() }),
        ),
        IpcCommand::ModbusSimStart => match protocol_control::start_modbus_runtime(state) {
            Ok(()) => ok_response(&request.id, json!({ "running": true })),
            Err(error) => protocol_error_response(&request.id, error),
        },
        IpcCommand::ModbusSimStop => match protocol_control::stop_modbus_tasks(state) {
            Ok(()) => ok_response(&request.id, json!({ "running": false })),
            Err(error) => protocol_error_response(&request.id, error),
        },
        IpcCommand::ModbusSimStatusGet => ok_response(
            &request.id,
            json!({ "running": state.is_modbus_running() }),
        ),
        IpcCommand::AppPing => ok_response(&request.id, json!({ "pong": true })),
        IpcCommand::AppVersionGet => ok_response(
            &request.id,
            json!({ "name": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION") }),
        ),
        _ => error_response(
            &request.id,
            IpcErrorCode::CmdUnknown,
            "Command not implemented in dispatcher yet",
        ),
    }
}

fn ok_response(id: &str, result: serde_json::Value) -> IpcResponse {
    IpcResponse {
        v: 1,
        kind: IpcMessageKind::Response,
        id: id.to_string(),
        ok: true,
        result: Some(result),
        error: None,
        ts: chrono::Utc::now().timestamp_millis(),
    }
}

fn error_response(id: &str, code: IpcErrorCode, message: &str) -> IpcResponse {
    IpcResponse {
        v: 1,
        kind: IpcMessageKind::Response,
        id: id.to_string(),
        ok: false,
        result: None,
        error: Some(IpcError {
            code,
            message: message.to_string(),
            details: None,
        }),
        ts: chrono::Utc::now().timestamp_millis(),
    }
}

fn protocol_error_response(id: &str, error: ProtocolControlError) -> IpcResponse {
    match error {
        ProtocolControlError::AlreadyRunning(message) | ProtocolControlError::NotRunning(message) => {
            error_response(id, IpcErrorCode::StateConflict, message)
        }
        ProtocolControlError::Operation(message) => {
            error_response(id, IpcErrorCode::InternalError, &message)
        }
        ProtocolControlError::Internal(message) => {
            error_response(id, IpcErrorCode::InternalError, &message)
        }
    }
}
