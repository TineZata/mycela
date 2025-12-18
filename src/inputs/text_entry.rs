use crate::state::{TextEntryConfigState, AlarmStatus};

pub fn render(config: &TextEntryConfigState, widget_id: &str) -> String {
    // Base64 encoded SVG icons for different alarm states
    const OFFLINE_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgd2lkdGg9IjI0IiBoZWlnaHQ9IjI0IiB2aWV3Qm94PSIwIDAgMjQgMjQiPjxwYXRoIGZpbGw9IiNmYTAwZmEiIHN0cm9rZT0iI2ZmZiIgc3Ryb2tlLWxpbmVqb2luPSJyb3VuZCIgc3Ryb2tlLWxpbmVjYXA9InJvdW5kIiBzdHJva2UtbWl0ZXJsaW1pdD0iNCIgc3Ryb2tlLXdpZHRoPSIxLjUiIGQ9Ik0yLjc1NyA2LjA5N2MwLTEuODQ1IDEuNDk2LTMuMzQgMy4zNC0zLjM0aDExLjgxOWMxLjg0NSAwIDMuMzQgMS40OTUgMy4zNCAzLjM0djExLjgxOWMwIDEuODQ1LTEuNDk1IDMuMzQtMy4zNCAzLjM0aC0xMS44MTljLTEuODQ1IDAtMy4zNC0xLjQ5NS0zLjM0LTMuMzR2LTExLjgxOXoiPjwvcGF0aD48cGF0aCBmaWxsPSJub25lIiBzdHJva2U9IiNmZmYiIHN0cm9rZS1saW5lam9pbj0icm91bmQiIHN0cm9rZS1saW5lY2FwPSJyb3VuZCIgc3Ryb2tlLW1pdGVybGltaXQ9IjQiIHN0cm9rZS13aWR0aD0iMS41IiBkPSJNMTcuODIgMTQuNDAyYzAuMTE2LTAuMjkzIDAuMTgtMC42MTEgMC4xOC0wLjk0NCAwLTEuMzY3LTEuMDc1LTIuNDktMi40NDgtMi42MTQtMC4yODEtMS42NjEtMS43NjQtMi45MjgtMy41NTItMi45MjgtMC4yNjggMC0wLjUyOSAwLjAyOC0wLjc4IDAuMDgyTTkuMTcyIDkuMjVjLTAuMzY5IDAuNDU0LTAuNjI0IDAuOTk5LTAuNzI1IDEuNTk1LTEuMzczIDAuMTI0LTIuNDQ4IDEuMjQ3LTIuNDQ4IDIuNjE0IDAgMS40NSAxLjIwOSAyLjYyNSAyLjcgMi42MjVoNi42YzAuMjc0IDAgMC41MzgtMC4wMzkgMC43ODctMC4xMTNNNi42IDYuNzVsMTAuOCAxMC41Ij48L3BhdGg+PC9zdmc+";
    
    const MAJOR_ALARM_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHZpZXdCb3g9IjAgMCAyMCAyMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48Y2lyY2xlIGN4PSIxMCIgY3k9IjEwIiByPSI4IiBmaWxsPSIjZmYwMDAwIi8+PHRleHQgeD0iMTAiIHk9IjE0IiB0ZXh0LWFuY2hvcj0ibWlkZGxlIiBmaWxsPSJ3aGl0ZSIgZm9udC1zaXplPSIxMiIgZm9udC13ZWlnaHQ9ImJvbGQiIGZvbnQtZmFtaWx5PSJBcmlhbCI+ITwvdGV4dD48L3N2Zz4=";
    
    const MINOR_ALARM_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHZpZXdCb3g9IjAgMCAyMCAyMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cGF0aCBkPSJNMTAgMyBMMTcgMTYgTDMgMTYgWiIgZmlsbD0iI2ZmYTUwMCIvPjx0ZXh0IHg9IjEwIiB5PSIxNCIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZmlsbD0id2hpdGUiIGZvbnQtc2l6ZT0iMTAiIGZvbnQtd2VpZ2h0PSJib2xkIiBmb250LWZhbWlseT0iQXJpYWwiPiE8L3RleHQ+PC9zdmc+";
    
    const INVALID_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHZpZXdCb3g9IjAgMCAyMCAyMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48Y2lyY2xlIGN4PSIxMCIgY3k9IjEwIiByPSI4IiBmaWxsPSIjOTk5OTk5Ii8+PHRleHQgeD0iMTAiIHk9IjE0IiB0ZXh0LWFuY2hvcj0ibWlkZGxlIiBmaWxsPSJ3aGl0ZSIgZm9udC1zaXplPSIxMiIgZm9udC13ZWlnaHQ9ImJvbGQiIGZvbnQtZmFtaWx5PSJBcmlhbCI+PzwvdGV4dD48L3N2Zz4=";
    
    // Determine border color and icon based on alarm status
    let (border_color, border_width, icon_html, status_text, readback_color) = match config.alarm_status {
        AlarmStatus::NotConnected => {
            let icon = format!("<img src='{}' alt='offline' style='position: absolute; left: 8px; top: 50%; transform: translateY(-50%); width: 20px; height: 20px;'>", OFFLINE_SVG);
            ("#fa00fa", "2px", icon, "DISCONNECTED", "#fa00fa")
        },
        AlarmStatus::Major => {
            let icon = format!("<img src='{}' alt='major alarm' style='position: absolute; left: 8px; top: 50%; transform: translateY(-50%); width: 20px; height: 20px;'>", MAJOR_ALARM_SVG);
            ("#ff0000", "2px", icon, "MAJOR ALARM", "#ff0000")
        },
        AlarmStatus::Minor => {
            let icon = format!("<img src='{}' alt='minor alarm' style='position: absolute; left: 8px; top: 50%; transform: translateY(-50%); width: 20px; height: 20px;'>", MINOR_ALARM_SVG);
            ("#ffa500", "2px", icon, "MINOR ALARM", "#ffa500")
        },
        AlarmStatus::Invalid => {
            let icon = format!("<img src='{}' alt='invalid' style='position: absolute; left: 8px; top: 50%; transform: translateY(-50%); width: 20px; height: 20px;'>", INVALID_SVG);
            ("#999999", "2px", icon, "INVALID", "#999999")
        },
        AlarmStatus::Normal => {
            ("#1e90ff", "1px", String::new(), "NORMAL", "#00cc66")
        },
    };

    // Check if we need left padding for icon
    let has_icon = !icon_html.is_empty();
    let input_padding_left = if has_icon { "padding-left: 35px;" } else { "padding-left: 12px;" };

    let input_bg_color = "#e6f3ff"; // light blue background
    let input_text_color = "#333";
    let pv_name = &config.pv_name;
    
    // Format the readback value
    let readback_display = if config.alarm_status == AlarmStatus::NotConnected {
        "---".to_string()
    } else if let Some(val) = config.current_value {
        format!("{:.2}", val)
    } else {
        "---".to_string()
    };

    if config.has_units {
        // With units on the right
        let units_bg = "#e8e8e8";
        format!(
            r##"<div class="text-entry-widget" data-widget-id="{widget_id}" data-pv="{pv_name}">
                <div style="margin-bottom: 4px; color: #888; font-size: 12px; font-family: monospace;">Setpoint:</div>
                <div style="display: flex; align-items: stretch; width: 100%;">
                    <div style="position: relative; flex: 1;">
                        {icon_html}
                        <input 
                            type="text" 
                            id="input-{widget_id}"
                            class="pv-input"
                            data-pv="{pv_name}"
                            style="background-color: {input_bg_color}; 
                                   color: {input_text_color}; 
                                   border: {border_width} solid {border_color}; 
                                   border-right: none;
                                   border-radius: 6px 0 0 6px; 
                                   padding: 10px; 
                                   {input_padding_left}
                                   font-size: 15px; 
                                   width: 100%; 
                                   box-sizing: border-box; 
                                   outline: none;">
                    </div>
                    <div style="background-color: {units_bg}; 
                               color: #666; 
                               border: {border_width} solid {border_color}; 
                               border-left: none;
                               border-radius: 0 6px 6px 0; 
                               padding: 10px 16px; 
                               font-size: 15px; 
                               display: flex; 
                               align-items: center; 
                               min-width: 50px;
                               justify-content: center;">{units}</div>
                </div>
                <div class="readback-row" style="display: flex; justify-content: space-between; align-items: center; margin-top: 8px; padding: 8px 12px; background: rgba(0,0,0,0.2); border-radius: 4px; border-left: 3px solid {readback_color};">
                    <span style="color: #888; font-size: 12px; font-family: monospace;">Readback:</span>
                    <span id="readback-{widget_id}" class="pv-readback" data-pv="{pv_name}" style="color: {readback_color}; font-size: 16px; font-weight: bold; font-family: monospace;">{readback_display} {units}</span>
                </div>
                <div style="display: flex; justify-content: space-between; margin-top: 4px;">
                    <span style="color: #666; font-size: 10px; font-family: monospace;">PV: {pv_name}</span>
                    <span id="status-{widget_id}" style="color: {readback_color}; font-size: 10px; font-family: monospace;">{status_text}</span>
                </div>
            </div>"##,
            widget_id = widget_id,
            pv_name = pv_name,
            icon_html = icon_html,
            input_bg_color = input_bg_color,
            input_text_color = input_text_color,
            border_width = border_width,
            border_color = border_color,
            input_padding_left = input_padding_left,
            units_bg = units_bg,
            units = config.units_text,
            readback_color = readback_color,
            readback_display = readback_display,
            status_text = status_text,
        )
    } else {
        // Without units - just the input field
        format!(
            r##"<div class="text-entry-widget" data-widget-id="{widget_id}" data-pv="{pv_name}">
                <div style="margin-bottom: 4px; color: #888; font-size: 12px; font-family: monospace;">Setpoint:</div>
                <div style="position: relative; width: 100%;">
                    {icon_html}
                    <input 
                        type="text" 
                        id="input-{widget_id}"
                        class="pv-input"
                        data-pv="{pv_name}"
                        style="background-color: {input_bg_color}; 
                               color: {input_text_color}; 
                               border: {border_width} solid {border_color}; 
                               border-radius: 6px; 
                               padding: 10px; 
                               {input_padding_left}
                               font-size: 15px; 
                               width: 100%; 
                               box-sizing: border-box; 
                               outline: none;">
                </div>
                <div class="readback-row" style="display: flex; justify-content: space-between; align-items: center; margin-top: 8px; padding: 8px 12px; background: rgba(0,0,0,0.2); border-radius: 4px; border-left: 3px solid {readback_color};">
                    <span style="color: #888; font-size: 12px; font-family: monospace;">Readback:</span>
                    <span id="readback-{widget_id}" class="pv-readback" data-pv="{pv_name}" style="color: {readback_color}; font-size: 16px; font-weight: bold; font-family: monospace;">{readback_display}</span>
                </div>
                <div style="display: flex; justify-content: space-between; margin-top: 4px;">
                    <span style="color: #666; font-size: 10px; font-family: monospace;">PV: {pv_name}</span>
                    <span id="status-{widget_id}" style="color: {readback_color}; font-size: 10px; font-family: monospace;">{status_text}</span>
                </div>
            </div>"##,
            widget_id = widget_id,
            pv_name = pv_name,
            icon_html = icon_html,
            input_bg_color = input_bg_color,
            input_text_color = input_text_color,
            border_width = border_width,
            border_color = border_color,
            input_padding_left = input_padding_left,
            readback_color = readback_color,
            readback_display = readback_display,
            status_text = status_text,
        )
    }
}


