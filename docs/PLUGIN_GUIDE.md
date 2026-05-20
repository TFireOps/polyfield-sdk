# Polyfield Plugin Development Guide

> 中文版本：[`PLUGIN_GUIDE.zh-CN.md`](PLUGIN_GUIDE.zh-CN.md)
> Field/method reference: [`API_REFERENCE.md`](API_REFERENCE.md)
> Architecture & internals: [`../CLAUDE.md`](../CLAUDE.md)

This guide walks you through writing, building, and shipping a Polyfield
plugin. After reading you should be able to write a non-trivial detection
plugin without reading the framework source.

---

## Contents

1. [Prerequisites](#1-prerequisites)
2. [Hello plugin](#2-hello-plugin)
3. [Core concepts](#3-core-concepts)
4. [Common patterns](#4-common-patterns)
5. [Build, install, run](#5-build-install-run)
6. [Limitations & gotchas](#6-limitations--gotchas)
7. [Reference & next steps](#7-reference--next-steps)

---

## 1. Prerequisites

- **Linux x86_64.** Plugins are `.so` files loaded into the game's IL2CPP
  process. Mac and Windows builds will not load — even cross-compiling
  to `x86_64-unknown-linux-gnu` from another host is fine, as long as
  the runtime is Linux.
- **stable Rust.** No nightly, no unstable features.
- **WSL2 if you're on Windows.** Recommended. Move your checkout into the
  WSL home filesystem (`~/`) instead of `/mnt/<drive>/...` — incremental
  builds are 5–10× faster.
- **A working framework deployment.** You need `libpolyfield.so` built
  and a `polyfield.toml` next to your game binary. See the framework
  [`README.md`](../README.md).

---

## 2. Hello plugin

### 2.1 Create the crate

```bash
cargo new --lib polyfield-hello
cd polyfield-hello
```

### 2.2 `Cargo.toml`

```toml
[package]
name = "polyfield-hello"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
polyfield = { path = "../Polyfield_AntiCheat/crates/sdk" }
# or, once published:
# polyfield = { git = "https://github.com/<owner>/polyfield-sdk" }
```

`crate-type = ["cdylib"]` is **required** — the framework loads plugins
via `dlopen` and looks up the C ABI entry point.

### 2.3 `src/lib.rs`

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
            description = "logs every player who joins",
        )
    }

    fn on_load(&mut self, ctx: &Ctx) {
        ctx.log_info("hello plugin online");
    }

    fn on_player_join(&mut self, evt: &mut PlayerJoinEvent, ctx: &Ctx) -> bool {
        ctx.log_info(&format!("welcome {}", evt.name));
        true   // forward the original RPC; return false to swallow it
    }
}

declare_plugin!(Hello::default());
```

### 2.4 Build & install

```bash
cargo build --release
cp target/release/libpolyfield_hello.so /path/to/Polyfield/plugins/
```

Restart the game. Look for `[hello] hello plugin online` in
`polyfield.log`.

That's the entire authoring surface: implement the `Plugin` trait, hand
the framework a constructor with `declare_plugin!`. Everything else is
optional.

---

## 3. Core concepts

### 3.1 The `Plugin` trait

Every callback has an empty default implementation. Override only the
events you care about. The trait requires `Send + Sync + 'static`
because the framework holds your plugin behind a vtable for the entire
process lifetime.

```rust
pub trait Plugin: Send + Sync + 'static {
    fn manifest() -> &'static PluginManifest where Self: Sized;

    // Lifecycle (notification — observe only)
    fn on_load(&mut self, _ctx: &Ctx) {}
    fn on_unload(&mut self, _ctx: &Ctx) {}        // see § 6, not invoked in v1

    // Interceptable — return false to skip the original RPC
    fn on_player_join(&mut self, _evt: &mut PlayerJoinEvent, _ctx: &Ctx) -> bool { true }
    fn on_damage(&mut self, _evt: &mut DamageEvent, _ctx: &Ctx) -> bool { true }
    fn on_chat(&mut self, _evt: &mut ChatEvent, _ctx: &Ctx) -> bool { true }

    // Notification
    fn on_latency(&mut self, _evt: &LatencySample, _ctx: &Ctx) {}
    fn on_tick(&mut self, _evt: &TickEvent, _ctx: &Ctx) {}
    fn on_game_start(&mut self, _evt: &GameStartEvent, _ctx: &Ctx) {}
}
```

### 3.2 Notification vs interceptable events

| Kind | Signature | What you can do |
|---|---|---|
| **Notification** | `&Event` | Read-only. Cannot prevent the game call. |
| **Interceptable** | `&mut Event → bool` | Mutate fields. Return `false` to skip the original RPC. |

Interceptability is a property of the underlying hook, not a stylistic
choice — an event is interceptable iff the framework can choose to skip
the corresponding game function.

**Multi-plugin rule.** Plugins are called in load order. The first
plugin that returns `false` short-circuits — remaining plugins are not
called and the original game function is not invoked.

There is **no `on_move` event by design.** Movement is polled inside
`on_tick` via `Player::position()`. This avoids a global per-player
diff loop in the framework even when no plugin cares about movement.

### 3.3 The three player types

| Type | Lifetime | Use for |
|---|---|---|
| `Player<'ctx>` | Tied to current callback | Reading any field, calling actions |
| `PlayerInfo` | Owned (`'static`) | Storing in `HashMap`, passing across callbacks |
| `PlayerRef` (`u64`) | Opaque key | `HashMap` key for cross-event tracking |

`Player<'ctx>` cannot outlive the `Ctx` it was built from — it carries a
borrowed pointer into the host vtable. Snapshot it with `player.info()`
or store the `PlayerRef` if you need to act later.

```rust
let p = evt.attacker(ctx);          // Player<'ctx>
let snapshot = p.info();            // PlayerInfo, owned
let key = evt.attacker;             // PlayerRef, copy

// Later, in another callback:
let p_again = ctx.player(key);      // rebuild Player<'ctx>
```

### 3.4 The `Ctx` handle

`Ctx` is the per-callback handle to the framework. It exposes:

- Logging: `log_info`, `log_warn`, `log_error`. Each line is prefixed
  with `[<your-plugin-name>]` in the unified `polyfield.log`.
- Host actions: `force_quit`, `show_dialog`.
- Player lookups: `player(ref)`, `host_player()`, `player_by_name()`,
  `player_by_id()`, `players()`.
- Game state: `game_map()`, `match_type()`, `entities_inspect()` —
  callable any time after the game has loaded.

`Ctx` is intentionally **not** `Send` or `Sync`. Do not store it, do not
move it to a thread. If you need to act outside the callback, save the
data you need (or a `PlayerRef`) and act from the next callback.

---

## 4. Common patterns

### 4.1 Damage validation

```rust
use polyfield::events::DamageEvent;
use polyfield::game_enums::DamageType;

fn on_damage(&mut self, evt: &mut DamageEvent, ctx: &Ctx) -> bool {
    let attacker = evt.attacker(ctx);

    // Don't moderate the host — they own the server.
    if attacker.is_host() {
        return true;
    }

    match evt.damage_type_enum() {
        Some(DamageType::Bullet) if evt.amount > 500 => {
            ctx.log_warn(&format!(
                "blocked impossible bullet damage: {} did {}",
                attacker.name(), evt.amount
            ));
            attacker.kick_with_reason("Banned", "Abnormal bullet damage.", 0.5);
            return false;       // swallow the RPC
        }
        _ => true,
    }
}
```

Mutating `evt.amount` before returning `true` would let the damage
through with a clamped value — useful for soft limits.

### 4.2 Chat moderation & commands

```rust
fn on_chat(&mut self, evt: &mut ChatEvent, ctx: &Ctx) -> bool {
    let sender = evt.sender(ctx);

    // /info command — log details and swallow the message.
    if evt.message.trim() == "/info" {
        ctx.log_info(&format!(
            "{} hp={} pos={:?}", sender.name(), sender.health(), sender.position()
        ));
        return false;
    }

    // Censor a word, allow the message through.
    if evt.message.to_lowercase().contains("badword") {
        evt.message = evt.message.replace("badword", "****");
    }

    // Block & punish.
    if evt.message.to_lowercase().contains("hack") {
        sender.kick_with_reason("Banned", "Banned word.", 0.5);
        return false;
    }

    true
}
```

`evt.message` is a `String` you can replace freely. The framework
allocates a fresh IL2CPP string only when you actually changed it.

### 4.3 Cross-event tracking with `PlayerRef`

```rust
use std::collections::HashMap;
use polyfield::events::{DamageEvent, PlayerRef};

#[derive(Default)]
struct DamageBudget {
    dealt_per_player: HashMap<PlayerRef, i64>,
}

impl Plugin for DamageBudget {
    // ... manifest ...

    fn on_damage(&mut self, evt: &mut DamageEvent, ctx: &Ctx) -> bool {
        let total = self.dealt_per_player.entry(evt.attacker).or_insert(0);
        *total += evt.amount as i64;

        if *total > 100_000 {
            let attacker = evt.attacker(ctx);
            attacker.kick_with_reason("Banned", "Damage budget exceeded.", 0.5);
            return false;
        }
        true
    }
}
```

Store `PlayerRef`, not `Player<'_>`. The latter has a lifetime; the
former is a plain `u64`.

### 4.4 Periodic polling via `on_tick`

`on_tick` fires once per server tick (default 50ms; tunable in
`polyfield.toml`'s `tick_interval_ms`). The frame counter is monotonic.

Use it for anything you'd want a periodic scheduler for:

```rust
fn on_tick(&mut self, evt: &TickEvent, ctx: &Ctx) {
    // Every tick — speed-hack check.
    for snap in ctx.players() {
        let p = ctx.player(snap.id);
        if p.is_dead() { continue; }
        if p.speed() > 20.0 && p.is_grounded() {
            ctx.log_warn(&format!("speed anomaly: {} {:.1}", p.name(), p.speed()));
        }
    }

    // Every ~1s (20 ticks) — cheaper checks.
    if evt.frame % 20 == 0 {
        // ...
    }
}
```

### 4.5 Teleport detection (movement diff)

There is no `on_move`. Diff positions across ticks instead:

```rust
use std::collections::HashMap;

#[derive(Default)]
struct Teleport {
    last_pos: HashMap<PlayerRef, [f32; 3]>,
}

const TICK_DT: f32 = 0.05;     // matches default tick_interval_ms

fn on_tick(&mut self, _evt: &TickEvent, ctx: &Ctx) {
    for snap in ctx.players() {
        let p = ctx.player(snap.id);
        if p.is_dead() { continue; }

        let pos = p.position();
        if let Some(prev) = self.last_pos.get(&snap.id).copied() {
            let d = distance(prev, pos);
            if d / TICK_DT > 50.0 {
                p.kick_with_reason("Kicked", "Teleport detected.", 0.5);
                continue;
            }
        }
        self.last_pos.insert(snap.id, pos);
    }
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0]-b[0], a[1]-b[1], a[2]-b[2]];
    (d[0]*d[0] + d[1]*d[1] + d[2]*d[2]).sqrt()
}
```

`Player::position()` reads the authoritative `_netTransform._recivedPos`
(falling back to `lastPlayerPos` when the net transform is unavailable).

### 4.6 Per-player actions

| Method | Backed by |
|---|---|
| `p.kick_me(delay)` | `Invoke("KickMePlz", delay)` |
| `p.kick_with_reason(title, body, delay)` | `show_error` then `kick_me` (recommended) |
| `p.kill()` | `RpcKillMe` |
| `p.set_health(h, flag)` | `RpcUpdateHealth` |
| `p.show_error(title, body)` | `RpcErrorPanel` |
| `p.is_host()` | `get_isLocalPlayer` |

**Always use `kick_with_reason` over raw `kick_me` when you want the
player to see why.** `show_error` must reach the client before the
disconnect; the convenience method does both with a sensible delay.

### 4.7 Detecting your own host

Skip moderation for the host — they own the server, and a buggy plugin
that kicks them ends the match for everyone:

```rust
if attacker.is_host() {
    return true;
}
```

### 4.8 Resetting state on a new game

`on_game_start` fires after `GameManager.Start` runs. By the time you
see it, `ctx.game_map()` and `ctx.match_type()` already reflect the new
match — perfect for clearing per-game tallies.

```rust
use polyfield::events::GameStartEvent;

fn on_game_start(&mut self, _evt: &GameStartEvent, ctx: &Ctx) {
    self.dealt_per_player.clear();
    self.last_pos.clear();
    ctx.log_info(&format!(
        "new game: {} ({})", ctx.game_map(), ctx.match_type()
    ));
}
```

The same accessors work anywhere — call `ctx.game_map()` from `on_tick`
or `on_damage` if a check should depend on the current map.

---

## 5. Build, install, run

### 5.1 Build

```bash
cargo build --release
# → target/release/lib<crate-name>.so
```

The shared object's name is `lib` + `[lib].name` from your `Cargo.toml`
(or `[package].name` with `-` replaced by `_`).

### 5.2 Install

Drop the `.so` into the framework's plugin directory (configured by
`plugins_dir` in `polyfield.toml`, default `plugins/`):

```bash
cp target/release/libpolyfield_hello.so /path/to/Polyfield/plugins/
```

### 5.3 Run

Plugins are scanned **once at game startup**. Restart the game to pick
up a rebuilt plugin.

```bash
RUST_LOG=info \
LD_PRELOAD=/path/to/libpolyfield.so \
./Polyfield.x86_64
```

Look for these lines in `polyfield.log`:

```
[polyfield] loading plugin: /path/to/plugins/libpolyfield_hello.so
[polyfield] registered: hello v0.1.0
[hello] hello plugin online
```

If you see `ABI version mismatch`, your plugin and the framework were
built against different SDK versions — rebuild against the SDK that
matches your framework binary.

---

## 6. Limitations & gotchas

- **No hot reload.** Plugins are scanned once on startup. Restart the
  game to pick up a rebuilt `.so`.
- **`on_unload` is not invoked in v1.** The game process typically
  exits without a clean teardown path. Don't rely on it for closing
  files or joining background threads.
- **All field reads can return defaults.** If the underlying
  `PlayerControl` is gone or a field couldn't be resolved on this game
  build, readers return `0` / `0.0` / `false` / empty string /
  `[0,0,0]` rather than `Option`. Sanity-check before acting on
  suspicious values.
- **Interception ordering is load order, first-`false`-wins.** If two
  plugins both want to moderate the same event, the one loaded first
  short-circuits the rest. Plan accordingly.
- **`ChatEvent` only fires for client→host messages** (hooks
  `UserCode_CmdSendChat__String`). The host's own local chat does not
  go through this RPC and is not seen by `on_chat`.
- **`Ctx` is not `Send`/`Sync`.** Do not store it across callbacks or
  move it to another thread. Save data instead, then rebuild a `Player`
  from the `PlayerRef` in the next callback.
- **ABI version gate.** The SDK pins `POLYFIELD_ABI_VERSION` (currently
  `13`); the framework refuses to load plugins compiled against a
  different value. Rebuild plugins whenever you upgrade the framework.
- **Adding new `PlayerControl` accessors does NOT bump ABI.** Adding new
  events, host actions, or changing event struct shapes does. Most
  upstream changes are non-breaking.
- **Don't do heavy work in interceptable hooks.** Damage / chat hooks
  run on the game thread before the RPC fires. Defer expensive logic
  to `on_tick` or your own background state.

---

## 7. Reference & next steps

- [`API_REFERENCE.md`](API_REFERENCE.md) — every method on `Player`,
  `Ctx`, `PlayerInfo`, plus the `DamageType` / `WeaponId` / `GadgetId`
  enums.
- [`../crates/plugin-example/`](../crates/plugin-example/) — working
  reference plugin: damage cap, chat moderation, `/info` command,
  teleport / speed-hack detection (commented out, ready to enable).
- [`../CLAUDE.md`](../CLAUDE.md) — framework architecture, how to add a
  new collector / event / host action / `PlayerControl` field.
- [`../polyfield.toml.example`](../polyfield.toml.example) — framework
  config template; documents `plugins_dir`, `tick_interval_ms`, the
  metadata dump, and host-level actions.
