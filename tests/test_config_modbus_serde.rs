// Copyright 2026 Tine Zata
// SPDX-License-Identifier: MPL-2.0

mod test_config_modbus_serde {
    use ctrl_sys_widgets::config::{ModbusTCPConfig, ModbusRegisterType, ScreenConfig};

    #[test]
    fn test_modbus_config_poll_interval_ms_alias_is_accepted() {
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
    fn test_modbus_config_missing_fields_use_correct_defaults() {
        let json = r#"{"host":"127.0.0.1","register":100,"register_type":"coil"}"#;
        let m: ModbusTCPConfig = serde_json::from_str(json).unwrap();
        assert_eq!(m.port, 502);
        assert_eq!(m.unit_id, 1);
        assert_eq!(m.min_poll_interval_ms, 500);
        assert_eq!(m.word_count, 1);
        assert_eq!(m.scale, 1.0);
        assert_eq!(m.offset, 0.0);
    }

    #[test]
    fn test_modbus_register_type_serde_roundtrip_for_all_variants() {
        let cases = [
            (ModbusRegisterType::HoldingRegister, "\"holding_register\""),
            (ModbusRegisterType::InputRegister, "\"input_register\""),
            (ModbusRegisterType::Coil, "\"coil\""),
            (ModbusRegisterType::DiscreteInput, "\"discrete_input\""),
        ];
        for (variant, expected_json) in cases {
            assert_eq!(serde_json::to_string(&variant).unwrap(), expected_json);
            let parsed: ModbusRegisterType = serde_json::from_str(expected_json).unwrap();
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn test_screen_config_load_returns_error_for_missing_file() {
        assert!(ScreenConfig::load("/nonexistent/path/config.json").is_err());
    }

    #[test]
    fn test_screen_config_deserializes_correctly_from_valid_json() {
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
    fn test_screen_config_validation_rejects_duplicate_widget_ids() {
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
}
