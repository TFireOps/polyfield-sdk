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

    fn on_player_join(&mut self, evt: &PlayerJoinEvent, ctx: &Ctx) {
        ctx.log_info(&format!("welcome {}", evt.name));
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
| `on_player_join(evt, ctx)` | Notification | Player joined / renamed |
| `on_reload(evt, ctx)` | Notification | Reload started |
| `on_latency(evt, ctx)` | Notification | Latency sample |
| `on_tick(evt, ctx)` | Notification | Every 50ms |
| `on_damage(evt, ctx) -> bool` | Interceptable | Damage dealt |
| `on_chat(evt, ctx) -> bool` | Interceptable | Chat message |
| `on_respawn(evt, ctx) -> bool` | Interceptable | Player respawned |
| `on_grenade(evt, ctx) -> bool` | Interceptable | Grenade thrown |
| `on_shoot(evt, ctx) -> bool` | Interceptable | Weapon fired |
| `on_vehicle_shoot(evt, ctx) -> bool` | Interceptable | Vehicle weapon fired |
| `on_vehicle_repair(evt, ctx) -> bool` | Interceptable | Vehicle repair completed |

**Interceptable**: return `true` to forward, `false` to block. Modify `evt` fields before returning `true` to alter the call. A panic in any callback is caught and fails open (the call is forwarded).

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
p.pos()            // Vec3 (distance math)
p.velocity()       // [f32; 3]
p.vel()            // Vec3
p.is_dead()        // bool
p.is_host()        // bool
p.ip()             // String
p.vehicle()        // Option<Vehicle> (vehicle they're in; None on foot)
p.is_in_vehicle()  // bool
p.kill()
p.kick_with_reason("Banned", "reason", 0.5)
p.set_health(100, 0)
p.show_error("Title", "Body")
p.send_chat_to("only you see this")   // directed chat
p.update_name("[3]newname")           // force display name
p.call_animation("Reloading")         // trigger animation
```

Raw field escape hatch (read any field with no dedicated getter yet):

```rust
use polyfield::fields;
let raw = p.read_raw_i32(fields::F_KILL_COUNT);
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
v.vehicle_type()   // Option<VehicleType> (coarse category)
v.model_name()     // String (specific model, e.g. "jagdpanther")
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

### KV store

Keys are auto-namespaced per plugin — no manual prefix needed:

```rust
ctx.kv_set("key", "value");                 // stored as "<plugin>:key"
if let Some(v) = ctx.kv_get("key") { /* ... */ }

// Deliberately shared across plugins:
ctx.kv_set_global("shared:key", "value");
let v = ctx.kv_get_global("shared:key");
```

### Server-wide scan (speed check)

`all_players()` returns **live handles** (not read-only snapshots), so you
can read team / crouch / speed on demand. This is what server-wide loops
(speed, load-gating, tallies) need:

```rust
fn on_tick(&mut self, evt: &TickEvent, ctx: &Ctx) {
    if evt.frame % 3 != 0 { return; }   // ~150ms

    for p in ctx.all_players() {
        if p.is_host() || p.is_dead() { continue; }
        let speed = p.vel().magnitude_2d();   // Vec3: horizontal speed
        let limit = match p.crouch() {
            0 => 13.0, 1 => 5.0, 2 => 4.0, _ => f32::INFINITY,
        };
        if speed > limit || speed > 25.0 {
            let n = self.over.entry(p.id()).or_insert(0);
            *n += 1;
            if *n >= 3 {
                p.kick_with_reason("Speed", &format!("speed={speed:.1}"), 0.5);
            }
        }
    }
}
```

### Backend channel (outbound + inbound)

`emit` sends a structured event to the management backend; `on_command`
receives commands from it and may reply:

```rust
// Outbound: report a cheat kick
ctx.emit("kickCheat", &format!(r#"{{"id":{},"reason":"speed"}}"#, p.player_id()));

// Inbound: backend invokes kick / ping
fn on_command(&mut self, name: &str, args: &str, ctx: &Ctx) -> Option<String> {
    match name {
        "ping" => Some("pong".to_string()),
        "kick" => {
            if let Some(p) = ctx.player_by_id(args.parse().ok()?) {
                p.kick_with_reason("Kicked", "by admin", 0.5);
            }
            None
        }
        _ => None,
    }
}
```

### Deferred actions / vote-map

`schedule_once` arms a one-shot timer that fires `on_timer`;
`set_current_time` rewrites the match countdown:

```rust
fn on_chat(&mut self, evt: &mut ChatEvent, ctx: &Ctx) -> bool {
    if evt.message == "/v" {
        self.votes += 1;
        if self.votes >= 15 {
            ctx.host_say(&color("red", "vote passed! rotating in 10s"));
            ctx.set_current_time(10.0);
        }
        return false;
    }
    true
}

// "warn now, re-check in 3s"
fn on_timer(&mut self, token: u64, ctx: &Ctx) {
    let suspect = ctx.player(token);   // token holds a PlayerRef
    if suspect.vel().magnitude_2d() > 25.0 {
        suspect.kick_with_reason("Speed", "still abnormal", 0.5);
    }
}
// at the detection site: ctx.schedule_once(3000, suspect.id());
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
