use crate::config::{ServerConfig, WidgetConfig};

pub fn setup_server_pvs(server: &pvxs_sys::Server, widgets: &[WidgetConfig]) -> pvxs_sys::Result<()> {
    for widget in widgets {
        if let Some(server_config) = &widget.server {
            let metadata = build_pv_metadata(server_config);
            match widget.data_type.as_deref() {
                Some("double") | Some("float") => {
                    tracing::info!("Creating DOUBLE PV: {}", widget.pv_name);
                    server.create_pv_double(&widget.pv_name, 1.0, metadata)?;
                }
                Some("int32") | Some("int") | Some("integer") => {
                    tracing::info!("Creating INT32 PV: {}", widget.pv_name);
                    server.create_pv_int32(&widget.pv_name, 0, metadata)?;
                }
                Some("string") | None => {
                    tracing::info!("Creating STRING PV: {}", widget.pv_name);
                    server.create_pv_string(&widget.pv_name, "", metadata)?;
                }
                Some(other) => {
                    tracing::warn!("Unknown data_type '{}' for {}, defaulting to STRING", other, widget.pv_name);
                    server.create_pv_string(&widget.pv_name, "", metadata)?;
                }
            }
            tracing::info!("✓ Added PV: {}", widget.pv_name);
        }
    }
    Ok(())
}

fn build_pv_metadata(server_config: &ServerConfig) -> pvxs_sys::NTScalarMetadataBuilder {
    let severity = server_config.alarm_serverity.as_ref()
        .map(|s| parse_alarm_severity(s))
        .unwrap_or(pvxs_sys::AlarmSeverity::NoAlarm);
    let status = server_config.alarm_status.as_ref()
        .map(|s| parse_alarm_status(s))
        .unwrap_or(pvxs_sys::AlarmStatus::NoAlarm);

    let mut builder = pvxs_sys::NTScalarMetadataBuilder::new()
        .alarm(severity, status, server_config.alarm_message.as_deref().unwrap_or(""));

    if let Some(metadata) = &server_config.metadata {
        if let Some(display) = &metadata.display {
            builder = builder.display(pvxs_sys::DisplayMetadata {
                limit_low: display.limit_low as i64,
                limit_high: display.limit_high as i64,
                description: display.description.clone(),
                units: display.units.clone(),
                precision: display.precision,
            });
        }
        if let Some(control) = &metadata.control {
            builder = builder.control(pvxs_sys::ControlMetadata {
                limit_low: control.limit_low,
                limit_high: control.limit_high,
                min_step: control.min_step,
            });
        }
        if let Some(alarm) = &metadata.alarm {
            builder = builder.alarm_metadata(pvxs_sys::AlarmMetadata {
                active: true,
                low_alarm_limit: alarm.low_alarm_limit,
                low_warning_limit: alarm.low_warning_limit,
                high_warning_limit: alarm.high_warning_limit,
                high_alarm_limit: alarm.high_alarm_limit,
                low_alarm_severity: parse_alarm_severity(&alarm.low_alarm_severity),
                low_warning_severity: parse_alarm_severity(&alarm.low_warning_severity),
                high_warning_severity: parse_alarm_severity(&alarm.high_warning_severity),
                high_alarm_severity: parse_alarm_severity(&alarm.high_alarm_severity),
                hysteresis: alarm.hysteresis as u8,
            });
        }
    }
    builder
}

fn parse_alarm_severity(severity: &str) -> pvxs_sys::AlarmSeverity {
    match severity.to_uppercase().as_str() {
        "NONE" => pvxs_sys::AlarmSeverity::NoAlarm,
        "MINOR" => pvxs_sys::AlarmSeverity::Minor,
        "MAJOR" => pvxs_sys::AlarmSeverity::Major,
        "INVALID" => pvxs_sys::AlarmSeverity::Invalid,
        _ => {
            tracing::warn!("Unknown alarm severity '{}', using NoAlarm", severity);
            pvxs_sys::AlarmSeverity::NoAlarm
        }
    }
}

fn parse_alarm_status(status: &str) -> pvxs_sys::AlarmStatus {
    match status.to_uppercase().as_str() {
        "NOALARM" | "NO_ALARM" | "NONE" => pvxs_sys::AlarmStatus::NoAlarm,
        "DEVICE" => pvxs_sys::AlarmStatus::DeviceStatus,
        "DRIVER" => pvxs_sys::AlarmStatus::DriverStatus,
        "RECORD" => pvxs_sys::AlarmStatus::RecordStatus,
        "DB" => pvxs_sys::AlarmStatus::DbStatus,
        "CONFIG" => pvxs_sys::AlarmStatus::ConfigStatus,
        "CLIENT" => pvxs_sys::AlarmStatus::ClientStatus,
        _ => {
            tracing::warn!("Unknown alarm status '{}', using DeviceStatus", status);
            pvxs_sys::AlarmStatus::DeviceStatus
        }
    }
}
