# Plugin Development Guide

> 中文版：[`PLUGIN_GUIDE.zh-CN.md`](PLUGIN_GUIDE.zh-CN.md)
> API reference: [`API_REFERENCE.md`](API_REFERENCE.md)

---

## 1. Prerequisites

- Linux x86_64 (WSL2 on Windows — put code in `~/`, not `/mnt/`)
- Stable Rust toolchain
- Framework binary `libpolyfield.so` + `polyfield.toml` next to the game

---

## 2. Hello Plugin

```bash
cargo new --lib my-plugin && cd my-plugin
```

`Cargo.toml`:

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

`src/lib.rs`:

```rust
use polyfield::{declare_plugin, manifest, Ctx, Plugin, PluginManifest};
use polyfield::events::PlayerJoinEvent;

#[derive(Default)]
struct Hello;

impl Plugin for Hello {
    fn manifest() -> &'static PluginManifest {
        manifest!(
            name = "hello",
            version = "0.1.0",
            authors = "you",
            description = "hello world",
        )
    }

    fn on_load(&mut self, ctx: &Ctx) {
        ctx.log_info("hello online");
    }

    fn on_player_join(&mut self, evt: &mut PlayerJoinEvent, ctx: &Ctx) -> bool {
        ctx.log_info(&format!("welcome {}", evt.name));
        true
    }
}

declare_plugin!(Hello::default());
```

Build & install:

```bash
cargo build --release
cp target/release/libmy_plugin.so /path/to/game/plugins/
```

---

## 3. Events

| Method | Type | Trigger |
|---|---|---|
| `on_load(ctx)` | Notification | Plugin loaded |
| `on_game_start(evt, ctx)` | Notification | New match started |
| `on_player_join(evt, ctx) -> bool` | Interceptable | Player joined |
| `on_damage(evt, ctx) -> bool` | Interceptable | Damage dealt |
| `on_chat(evt, ctx) -> bool` | Interceptable | Chat message |
| `on_respawn(evt, ctx) -> bool` | Interceptable | Player respawned |
| `on_grenade(evt, ctx) -> bool` | Interceptable | Grenade thrown |
| `on_shoot(evt, ctx) -> bool` | Interceptable | Weapon fired |
| `on_vehicle_shoot(evt, ctx) -> bool` | Interceptable | Vehicle weapon fired |
| `on_vehicle_repair(evt, ctx) -> bool` | Interceptable | Vehicle repair completed |
| `on_reload(evt, ctx)` | Notification | Reload started |
| `on_latency(evt, ctx)` | Notification | Latency sample |
| `on_tick(evt, ctx)` | Notification | Every 50ms |

**Interceptable**: return `true` to forward, `false` to block. Modify `evt` fields before returning `true` to alter the call.

---

## 4. Player & Vehicle

### Player

```rust
let p = evt.player(ctx);
let p = ctx.player(ref);
let p = ctx.host_player();
let p = ctx.player_by_name("xx");
let p = ctx.player_by_id(3);
```

Key methods:

```rust
p.name()           // String
p.player_id()      // u32 (slot)
p.health()         // i32
p.position()       // [f32; 3]
p.velocity()       // [f32; 3]
p.is_dead()        // bool
p.is_host()        // bool
p.ip()             // String
p.kill()
p.kick_with_reason("Banned", "reason", 0.5)
p.set_health(100, 0)
p.show_error("Title", "Body")
```

### Vehicle

```rust
let v = evt.vehicle(ctx);
let v = ctx.vehicle(ref);
for v in ctx.vehicles() { }
```

Key methods:

```rust
v.health()         // i32
v.vehicle_type()   // Option<VehicleType>
v.position()       // [f32; 3]
v.velocity()       // [f32; 3]
v.rotation()       // [f32; 3]
v.driver()         // Option<Player>
```

### Cross-event tracking

Store `PlayerRef` (u64), rebuild `Player` when needed:

```rust
let key: PlayerRef = evt.player;
// later:
let p = ctx.player(key);
```

---

## 5. Examples

### Damage validation

```rust
fn on_damage(&mut self, evt: &mut DamageEvent, ctx: &Ctx) -> bool {
    let attacker = evt.attacker(ctx);
    if attacker.is_host() { return true; }
    if evt.amount > 500 {
        attacker.kick_with_reason("Banned", "abnormal damage", 0.5);
        return false;
    }
    true
}
```

### Chat moderation

```rust
fn on_chat(&mut self, evt: &mut ChatEvent, ctx: &Ctx) -> bool {
    if evt.message.contains("badword") {
        evt.message = evt.message.replace("badword", "****");
    }
    true
}
```

### Periodic checks

```rust
fn on_tick(&mut self, evt: &TickEvent, ctx: &Ctx) {
    if evt.frame % 100 == 0 {  // every 5s
        ctx.log_info(&format!("players: {}", ctx.players().len()));
    }
}
```

### Host chat & colors

```rust
use polyfield::color;
ctx.host_say(&format!("{} welcome!", color("red", "Server")));
```

### Shared KV store

```rust
ctx.kv_set("myplugin:key", "value");
if let Some(v) = ctx.kv_get("myplugin:key") { /* ... */ }
```

---

## 6. Build & Run

```bash
cargo build --release
cp target/release/libmy_plugin.so /path/to/game/plugins/
RUST_LOG=info LD_PRELOAD=/path/to/libpolyfield.so ./Polyfield.x86_64
```

---

## 7. Gotchas

- No hot reload — restart the game after rebuilding
- Field reads never fail — return `0` / `false` / `""` / `[0,0,0]` on error
- `Ctx` is not `Send`/`Sync` — don't store it, don't move it to threads
- Interceptable hooks run on the game thread — keep them fast
- `on_chat` doesn't see the host's own messages
- Multi-plugin: first `false` wins, remaining plugins are skipped
