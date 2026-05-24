// Copyright 2026 Tine Zata
// SPDX-License-Identifier: MPL-2.0

mod test_config_widget_config {
    use mycelo::config::{
        EpicsPvaConfig, ModbusTCPConfig, ModbusRegisterType, ProtocolConfig,
        WidgetConfig, WidgetType,
    };

    fn widget(id: &str, widget_type: WidgetType) -> WidgetConfig {
        WidgetConfig {
            id: id.to_string(),
            widget_type,
            label: format!("{id} label"),
            protocol: None,
            data_type: None,
            description: None,
            style: None,
            options: None,
            orientation: None,
            level: None,
            children: None,
            max_points: None,
            chart_type: None,
            axis_label_x: None,
            axis_label_y: None,
            size: None,
            metadata: None,
        }
    }

    #[test]
    fn test_epics_pva_protocol_produces_pv_name_as_channel_address() {
        let mut w = widget("w1", WidgetType::TextUpdate);
        w.protocol = Some(ProtocolConfig::EpicsPva(EpicsPvaConfig {
            pv_name: "test:pv".to_string(),
            server: None,
            pv_names: None,
        }));
        assert_eq!(w.channel_address(), "test:pv");
    }

    #[test]
    fn test_modbus_tcp_protocol_produces_url_as_channel_address() {
        let mut w = widget("w2", WidgetType::Gauge);
        w.protocol = Some(ProtocolConfig::ModbusTcp(ModbusTCPConfig {
            host: "127.0.0.1".to_string(),
            port: 502,
            unit_id: 1,
            register: 1000,
            register_type: ModbusRegisterType::HoldingRegister,
            min_poll_interval_ms: 500,
            scale: 1.0,
            offset: 0.0,
            word_count: 1,
        }));
        assert_eq!(w.channel_address(), "modbus-tcp://127.0.0.1:502/reg1000");
    }

    #[test]
    fn test_no_protocol_configured_returns_empty_channel_address() {
        assert_eq!(widget("w3", WidgetType::TextUpdate).channel_address(), "");
    }

    #[test]
    fn test_epics_pva_accessor_returns_some_and_modbus_returns_none() {
        let mut w = widget("e", WidgetType::TextUpdate);
        w.protocol = Some(ProtocolConfig::EpicsPva(EpicsPvaConfig {
            pv_name: "x:pv".to_string(),
            server: None,
            pv_names: None,
        }));
        assert!(w.epics_pva().is_some());
        assert!(w.modbus_tcp().is_none());
    }

    #[test]
    fn test_modbus_tcp_accessor_returns_some_and_epics_returns_none() {
        let mut w = widget("m", WidgetType::Gauge);
        w.protocol = Some(ProtocolConfig::ModbusTcp(ModbusTCPConfig {
            host: "127.0.0.1".to_string(),
            port: 502,
            unit_id: 1,
            register: 1000,
            register_type: ModbusRegisterType::HoldingRegister,
            min_poll_interval_ms: 500,
            scale: 1.0,
            offset: 0.0,
            word_count: 1,
        }));
        assert!(w.modbus_tcp().is_some());
        assert!(w.epics_pva().is_none());
    }

    #[test]
    fn test_series_pvs_with_only_primary_pv_returns_single_element() {
        let e = EpicsPvaConfig {
            pv_name: "main:pv".to_string(),
            server: None,
            pv_names: None,
        };
        assert_eq!(e.series_pvs(), vec!["main:pv"]);
    }

    #[test]
    fn test_series_pvs_with_additional_pv_names_returns_all_combined() {
        let e = EpicsPvaConfig {
            pv_name: "main:pv".to_string(),
            server: None,
            pv_names: Some(vec!["e1".to_string(), "e2".to_string()]),
        };
        assert_eq!(e.series_pvs(), vec!["main:pv", "e1", "e2"]);
    }

    #[test]
    fn test_series_pvs_capped_at_six_total_including_primary() {
        let e = EpicsPvaConfig {
            pv_name: "main:pv".to_string(),
            server: None,
            pv_names: Some((0..10).map(|i| format!("extra:{i}")).collect()),
        };
        assert_eq!(e.series_pvs().len(), 6);
    }
}
