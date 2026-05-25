# Configuration Error Handling

This document describes the improved error handling for configuration file loading in mycela.

## Overview

The configuration loader now provides detailed, helpful error messages when there are problems with your `demo_config.json` or other configuration files. Instead of generic panic messages, you'll get:

- **File location** (path)
- **Line and column numbers** where the error occurred
- **Context** showing the problematic lines with surrounding code
- **Specific error description** from the JSON parser
- **Helpful hints** about how to fix common issues

## Error Types

### Missing Required Fields

When a required field is missing, you'll see exactly which field and where:

```
❌ FAILED to load configuration:

Configuration JSON error: missing field `label` at line 11 column 5
📄 File: examples/test_missing_label.json
📍 Line: 11, Column: 5

Context:
    9:       "type": "text_update",
    10:       "data_type": "double"
  ➤ 11:     }
    12:   ]
    13: }

❌ Error: missing field `label` at line 11 column 5

💡 Hint: Each widget must have a 'label' field for display (string).
```

**Required fields:**
- Root config: `id`, `title`, `description`, `widgets`
- Each widget: `id`, `pv_name`, `type`, `label`

### Invalid Widget Type

When you use an unknown widget type:

```
❌ FAILED to load configuration:

Configuration JSON error: unknown variant `invalid_type`, expected one of `text_entry`, `text_update`, `gauge`, `led`, `button`, `slider`, `chart` at line 9 column 28
📄 File: examples/test_invalid_type.json
📍 Line: 9, Column: 28

Context:
    7:       "id": "widget1",
    8:       "pv_name": "demo:test:pv",
  ➤ 9:       "type": "invalid_type",
    10:       "label": "Test Widget",
    11:       "data_type": "double"

❌ Error: unknown variant `invalid_type`, expected one of `text_entry`, `text_update`, `gauge`, `led`, `button`, `slider`, `chart` at line 9 column 28

💡 Hint: Check for typos in field names or enum values.
   Valid widget types: text_entry, text_update, gauge, led, button, slider, chart
```

**Valid widget types:**
- `text_entry` - Text input field for setting PV values
- `text_update` - Read-only text display
- `gauge` - Visual gauge/meter display
- `led` - LED indicator
- `button` - Button control
- `slider` - Slider control
- `chart` - Time-series chart

### Duplicate Widget IDs

Each widget must have a unique ID:

```
❌ FAILED to load configuration:

Configuration JSON error: invalid type: string "duplicate_id", expected unit at line 1 column 14
⚠️  Widget #2 has duplicate ID: 'widget1'
💡 Each widget must have a unique 'id' field.
```

### JSON Syntax Errors

Standard JSON syntax errors (missing commas, brackets, quotes, etc.) will show the exact location and context.

## Testing Your Configuration

You can test your configuration file without running the full server:

```bash
cargo run --bin test-config-load examples/demo_config.json
```

This will quickly validate your config and show any errors with detailed context.

### Example Test Files

Several test configuration files are provided to demonstrate error handling:

- `examples/test_missing_label.json` - Missing required field
- `examples/test_invalid_type.json` - Invalid widget type
- `examples/test_duplicate_id.json` - Duplicate widget IDs

Try loading these to see the error messages:

```bash
cargo run --bin test-config-load examples/test_missing_label.json
cargo run --bin test-config-load examples/test_invalid_type.json
cargo run --bin test-config-load examples/test_duplicate_id.json
```

## Implementation Details

The error handling is implemented in [config.rs](../src/config.rs):

- Custom `ConfigError` type with detailed context
- Line-by-line context extraction from the JSON file
- Field-specific hints for common errors
- Validation of widget ID uniqueness

The panic-free error handling ensures that configuration problems are caught early with actionable feedback.
