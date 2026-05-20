# Polyfield SDK

Plugin SDK for the Polyfield anti-cheat framework. Write detection plugins in Rust, compile to `.so`, drop into the `plugins/` directory.

## Quick Start

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

```bash
cargo build --release
cp target/release/libmy_plugin.so /path/to/game/plugins/
```

## Documentation

- [Plugin Guide (English)](docs/PLUGIN_GUIDE.md)
- [插件开发指南 (中文)](docs/PLUGIN_GUIDE.zh-CN.md)
- [API Reference](docs/API_REFERENCE.md)
- [Example Plugin](examples/plugin-example/)

## Requirements

- Linux x86_64 runtime
- Stable Rust toolchain
- The Polyfield framework binary (`libpolyfield.so`)

## License

MIT
