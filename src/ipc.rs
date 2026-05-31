use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IpcMessageKind {
    Request,
    Response,
    Event,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IpcCommand {
    EpicsServerStart,
    EpicsServerStop,
    EpicsServerStatusGet,
    EpicsPvRead,
    EpicsPvWrite,
    EpicsPvSubscribe,
    EpicsPvUnsubscribe,
    ModbusSimStart,
    ModbusSimStop,
    ModbusSimStatusGet,
    ModbusRead,
    ModbusWrite,
    ModbusSubscribe,
    ModbusUnsubscribe,
    AppPing,
    AppVersionGet,
}

impl IpcCommand {
    pub fn is_mutating(&self) -> bool {
        matches!(
            self,
            Self::EpicsServerStart
                | Self::EpicsServerStop
                | Self::EpicsPvWrite
                | Self::ModbusSimStart
                | Self::ModbusSimStop
                | Self::ModbusWrite
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    pub v: u16,
    pub kind: IpcMessageKind,
    pub id: String,
    pub cmd: IpcCommand,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub payload: Value,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    pub code: IpcErrorCode,
    pub message: String,
    #[serde(default)]
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IpcErrorCode {
    AuthInvalidToken,
    AuthExpired,
    CmdUnknown,
    PayloadInvalid,
    StateConflict,
    InternalError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub v: u16,
    pub kind: IpcMessageKind,
    pub id: String,
    pub ok: bool,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<IpcError>,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcEvent {
    pub v: u16,
    pub kind: IpcMessageKind,
    pub event: String,
    pub data: Value,
    pub ts: i64,
}
