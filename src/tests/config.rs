use super::*;
use crate::test_helpers as th;

// ── AlarmMetadata::compute_severity ────────────────────────────────────

#[test]
fn alarm_below_low_alarm_is_major() {
    assert_eq!(th::alarm_meta().compute_severity(5.0), 2);
}

#[test]
fn alarm_above_high_alarm_is_major() {
    assert_eq!(th::alarm_meta().compute_severity(95.0), 2);
}

#[test]
fn alarm_below_low_warning_is_minor() {
    // 15.0 is between low_alarm(10) and low_warning(20) → MINOR
    assert_eq!(th::alarm_meta().compute_severity(15.0), 1);
}

#[test]
fn alarm_above_high_warning_is_minor() {
    assert_eq!(th::alarm_meta().compute_severity(85.0), 1);
}

#[test]
fn alarm_in_normal_range_is_zero() {
    assert_eq!(th::alarm_meta().compute_severity(50.0), 0);
}

#[test]
fn alarm_exactly_at_low_alarm_limit_is_warning() {
    // value == low_alarm_limit(10.0) does NOT trigger alarm (uses <),
    // but IS < low_warning_limit(20.0), so it returns MINOR (1)
    assert_eq!(th::alarm_meta().compute_severity(10.0), 1);
}

#[test]
fn alarm_exactly_at_high_alarm_limit_is_warning() {
    // value == high_alarm_limit(90.0) does NOT trigger alarm (uses >),
    // but IS > high_warning_limit(80.0), so it returns MINOR (1)
    assert_eq!(th::alarm_meta().compute_severity(90.0), 1);
}

#[test]
fn alarm_severity_string_unknown_yields_zero() {
    let m = AlarmMetadata {
        low_alarm_limit: 10.0,
        low_warning_limit: 20.0,
        high_warning_limit: 80.0,
        high_alarm_limit: 90.0,
        low_alarm_severity: "BADVAL".to_string(),
        low_warning_severity: "".to_string(),
        high_warning_severity: "".to_string(),
        high_alarm_severity: "BADVAL".to_string(),
        hysteresis: 0,
    };
    assert_eq!(m.compute_severity(5.0), 0);
    assert_eq!(m.compute_severity(95.0), 0);
}

// ── WidgetConfig helpers ────────────────────────────────────────────────

#[test]
fn channel_address_epics() {
    let mut w = th::widget("w1", WidgetType::TextUpdate);
    w.protocol = Some(ProtocolConfig::EpicsPva(EpicsPvaConfig {
        pv_name: "test:pv".to_string(),
        server: None,
        pv_names: None,
    }));
    assert_eq!(w.channel_address(), "test:pv");
}

#[test]
fn channel_address_modbus() {
    let mut w = th::widget("w2", WidgetType::Gauge);
    w.protocol = Some(ProtocolConfig::ModbusTcp(th::modbus_cfg(1.0, 0.0)));
    assert_eq!(w.channel_address(), "modbus-tcp://127.0.0.1:502/reg1000");
}

#[test]
fn channel_address_none_is_empty() {
    assert_eq!(th::widget("w3", WidgetType::TextUpdate).channel_address(), "");
}

#[test]
fn epics_pva_returns_some_for_epics_widget() {
    let mut w = th::widget("e", WidgetType::TextUpdate);
    w.protocol = Some(ProtocolConfig::EpicsPva(EpicsPvaConfig {
        pv_name: "x:pv".to_string(),
        server: None,
        pv_names: None,
    }));
    assert!(w.epics_pva().is_some());
    assert!(w.modbus_tcp().is_none());
}

#[test]
fn modbus_tcp_returns_some_for_modbus_widget() {
    let mut w = th::widget("m", WidgetType::Gauge);
    w.protocol = Some(ProtocolConfig::ModbusTcp(th::modbus_cfg(1.0, 0.0)));
    assert!(w.modbus_tcp().is_some());
    assert!(w.epics_pva().is_none());
}

// ── EpicsPvaConfig::series_pvs ──────────────────────────────────────────

#[test]
fn series_pvs_primary_only() {
    let e = EpicsPvaConfig { pv_name: "main:pv".to_string(), server: None, pv_names: None };
    assert_eq!(e.series_pvs(), vec!["main:pv"]);
}

#[test]
fn series_pvs_with_extras() {
    let e = EpicsPvaConfig {
        pv_name: "main:pv".to_string(),
        server: None,
        pv_names: Some(vec!["e1".to_string(), "e2".to_string()]),
    };
    assert_eq!(e.series_pvs(), vec!["main:pv", "e1", "e2"]);
}

#[test]
fn series_pvs_capped_at_six_total() {
    let e = EpicsPvaConfig {
        pv_name: "main:pv".to_string(),
        server: None,
        pv_names: Some((0..10).map(|i| format!("extra:{i}")).collect()),
    };
    assert_eq!(e.series_pvs().len(), 6); // primary + max 5 extras
}

// ── Serde deserialization ───────────────────────────────────────────────

#[test]
fn modbus_config_accepts_old_poll_interval_alias() {
    let json = r#"{
        "host": "127.0.0.1",
        "register": 100,
        "register_type": "holding_register",
        "poll_interval_ms": 1000
    }"#;
    let m: ModbusTCPConfig = serde_json::from_str(json).unwrap();
    assert_eq!(m.min_poll_interval_ms, 1000);
}

#[test]
fn modbus_config_defaults_applied_when_absent() {
    let json = r#"{"host":"127.0.0.1","register":100,"register_type":"coil"}"#;
    let m: ModbusTCPConfig = serde_json::from_str(json).unwrap();
    assert_eq!(m.port, 502);
    assert_eq!(m.unit_id, 1);
    assert_eq!(m.min_poll_interval_ms, 500);
    assert_eq!(m.word_count, 1);
    assert!((m.scale - 1.0).abs() < f64::EPSILON);
    assert!((m.offset - 0.0).abs() < f64::EPSILON);
}

#[test]
fn modbus_register_type_serde_roundtrip() {
    let cases = [
        (ModbusRegisterType::HoldingRegister, "\"holding_register\""),
        (ModbusRegisterType::InputRegister,   "\"input_register\""),
        (ModbusRegisterType::Coil,            "\"coil\""),
        (ModbusRegisterType::DiscreteInput,   "\"discrete_input\""),
    ];
    for (variant, expected_json) in cases {
        assert_eq!(serde_json::to_string(&variant).unwrap(), expected_json);
        let parsed: ModbusRegisterType = serde_json::from_str(expected_json).unwrap();
        assert_eq!(parsed, variant);
    }
}

#[test]
fn screen_config_load_missing_file_returns_error() {
    assert!(ScreenConfig::load("/nonexistent/path/config.json").is_err());
}

#[test]
fn screen_config_deserializes_from_json() {
    let json = r#"{
        "id": "screen1",
        "title": "Test Screen",
        "description": "A test screen",
        "widgets": [{
            "id": "w1", "type": "text_update", "label": "Value",
            "protocol": {"type": "epics-pva", "pv_name": "test:double"}
        }]
    }"#;
    let sc: ScreenConfig = serde_json::from_str(json).unwrap();
    assert_eq!(sc.id, "screen1");
    assert_eq!(sc.widgets.len(), 1);
    assert_eq!(sc.widgets[0].id, "w1");
}

#[test]
fn screen_config_rejects_duplicate_widget_ids() {
    let json = r#"{
        "id": "s", "title": "T", "description": "D",
        "widgets": [
            {"id": "dup", "type": "text_update", "label": "A",
             "protocol": {"type": "epics-pva", "pv_name": "a:pv"}},
            {"id": "dup", "type": "text_update", "label": "B",
             "protocol": {"type": "epics-pva", "pv_name": "b:pv"}}
        ]
    }"#;
    let sc: ScreenConfig = serde_json::from_str(json).unwrap();
    assert!(ScreenConfig::validate_config(&sc).is_err());
}
