# Polyfield SDK

Plugin SDK for the Polyfield anti-cheat framework. Write detection plugins in Rust, compile to `.so`, drop into the `plugins/` directory.

## Installation

### 1. Download the framework

Download `libpolyfield.so` from the [Releases](https://github.com/TFireOps/polyfield-sdk/releases) page and place it next to your game binary.

### 2. Create config file

Create `polyfield.toml` next to the game binary:

```toml
plugins_dir = "plugins"
tick_interval_ms = 50

[dump]
enabled = false
dir = "dump"
mode = "single"

# Block the game from announcing to the public server list
# [server_list]
# block_share = true
# custom_fields = false
# name = "{mapName} - {gamemode}"
# region = "CN"

# Network optimisation (fixes NetworkWriterPool errors + speeds up map load)
# [network_optim]
# enabled = true
```

### 3. Create plugins directory

```bash
mkdir plugins
```

### 4. Launch

```bash
RUST_LOG=info LD_PRELOAD=$PWD/libpolyfield.so ./Polyfield.x86_64
```

The framework will:
1. Wait for `GameAssembly.so` to load
2. Initialize the IL2CPP bridge
3. Install event hooks (damage, chat, respawn, etc.)
4. Load all `.so` files from `plugins/`
5. Log to `polyfield.log`

---

## Writing a Plugin

### Cargo.toml

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
polyfield = { git = "https://github.com/TFireOps/polyfield-sdk" }
```

### src/lib.rs

```rust
use polyfield::{declare_plugin, manifest, Ctx, Plugin, PluginManifest};
use polyfield::events::PlayerJoinEvent;

#[derive(Default)]
struct MyPlugin;

impl Plugin for MyPlugin {
    fn manifest() -> &'static PluginManifest {
        manifest!(
            name = "my-plugin",
            version = "0.1.0",
            authors = "you",
            description = "my first plugin",
        )
    }

    fn on_load(&mut self, ctx: &Ctx) {
        ctx.log_info("online");
    }

    fn on_player_join(&mut self, evt: &mut PlayerJoinEvent, ctx: &Ctx) -> bool {
        ctx.log_info(&format!("welcome {}", evt.name));
        true
    }
}

declare_plugin!(MyPlugin::default());
```

### Build & Deploy

```bash
cargo build --release
cp target/release/libmy_plugin.so /path/to/game/plugins/
```

Restart the game. Check `polyfield.log` for your plugin's output.

---

## Configuration Reference

| Key | Default | Description |
|---|---|---|
| `plugins_dir` | `"plugins"` | Directory to scan for plugin `.so` files |
| `tick_interval_ms` | `50` | `on_tick` interval in milliseconds |
| `[dump] enabled` | `false` | Dump IL2CPP metadata to disk |
| `[server_list] block_share` | `false` | Block public server list announcement |
| `[server_list] custom_fields` | `false` | Rewrite server list fields with templates |
| `[network_optim] enabled` | `false` | Enable Mirror network fixes |

---

## Documentation

- [Plugin Guide (English)](docs/PLUGIN_GUIDE.md)
- [插件开发指南 (中文)](docs/PLUGIN_GUIDE.zh-CN.md)
- [API Reference](docs/API_REFERENCE.md)
- [Example Plugin](examples/plugin-example/)

## Requirements

- Linux x86_64 runtime
- Stable Rust toolchain (for building plugins)
- The Polyfield framework binary (`libpolyfield.so`) — download from [Releases](https://github.com/TFireOps/polyfield-sdk/releases)

## License

MIT
