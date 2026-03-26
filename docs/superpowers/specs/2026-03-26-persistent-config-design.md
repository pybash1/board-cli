# Persistent Config System Design

**Date**: 2026-03-26
**Author**: System Design
**Status**: Approved

## Overview

Replace environment variable-based device code storage with a persistent TOML configuration file that can be shared between CLI and TUI interfaces.

## Problem Statement

Currently, device codes are stored in environment variables (`BOARD_DEVICE_CODE`) which are lost when the application exits. Users must re-register devices or manually set environment variables every time they restart the application.

## Solution

Implement a centralized ConfigManager that handles persistent storage of device codes along with application and API settings in a TOML configuration file.

## Design Specifications

### Configuration Structure

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    // Existing app settings
    pub data_dir: PathBuf,
    pub theme: String,
    pub auto_save: bool,

    // Device authentication
    pub device_code: Option<DeviceCode>,

    // API settings
    pub api_base_url: String,
    pub api_timeout_seconds: u64,
    pub max_retries: u32,
}
```

### ConfigManager Architecture

```rust
pub struct ConfigManager {
    config_path: PathBuf,
    config: Arc<RwLock<AppConfig>>,
}
```

**Key Features:**
- Singleton pattern for consistent access
- Thread-safe with Arc<RwLock<>> for TUI usage
- Automatic config file creation and directory setup
- Atomic writes to prevent corruption
- Graceful fallback to defaults on errors

### File Location and Format

**Location**: `~/.config/board-cli/config.toml`

**TOML Structure**:
```toml
[app]
data_dir = "/home/user/.board-cli"
theme = "default"
auto_save = true

[device]
code = "abc123xyz789"

[api]
base_url = "https://api.board.example.com"
timeout_seconds = 30
max_retries = 3
```

### Integration Points

#### CLI Integration
- Replace environment variable checks with ConfigManager
- Auto-save device code after successful registration
- Load all API settings from config
- Support config reset/clear commands

#### TUI Integration
- Shared ConfigManager instance across components
- Real-time config updates in settings screens
- No separate persistence logic needed

#### API Client Integration
- BoardClient accepts config reference in constructor
- Automatic device code loading
- Configurable timeouts and retry behavior
- Fallback registration flow if no device code

### Error Handling

| Scenario | Behavior |
|----------|----------|
| Missing config file | Create with defaults on first save |
| Corrupted TOML | Backup file, use defaults, log warning |
| Permission errors | Fallback to env vars, warn user |
| Invalid device code | Clear code, trigger re-registration |
| Network config issues | Use built-in defaults |

### Implementation Plan

1. **Update Dependencies**: Add `toml` crate
2. **Enhance AppConfig**: Add device_code and API fields
3. **Create ConfigManager**: Implement load/save/update methods
4. **Update API Client**: Accept config in constructor
5. **Modify CLI Commands**: Use ConfigManager instead of env vars
6. **Update TUI**: Integrate ConfigManager for device persistence
7. **Add Error Handling**: Graceful degradation for all error cases

### Testing Strategy

- Unit tests for ConfigManager load/save operations
- Integration tests for CLI/TUI config sharing
- Error handling tests for corrupted/missing files
- Migration tests from env var to config file

### Migration Path

For users with existing `BOARD_DEVICE_CODE` environment variables:
1. Check for existing env var on first run
2. If found, migrate to config file
3. Log migration completion
4. Continue normal operation

## Success Criteria

- Device codes persist between application restarts
- CLI and TUI share configuration consistently
- No data loss during config file operations
- Graceful handling of all error conditions
- Smooth migration from environment variables