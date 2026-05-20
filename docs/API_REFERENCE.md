# API Reference

Detailed reference for the Polyfield plugin SDK. For architecture overview
see [`../CLAUDE.md`](../CLAUDE.md).

---

## Plugin trait

```rust
pub trait Plugin: Send + Sync + 'static {
    fn manifest() -> &'static PluginManifest where Self: Sized;

    // Lifecycle (notification)
    fn on_load(&mut self, ctx: &Ctx) {}
    fn on_unload(&mut self, ctx: &Ctx) {}

    // Interceptable events (mutate params, return false to block original RPC)
    fn on_player_join(&mut self, evt: &mut PlayerJoinEvent, ctx: &Ctx) -> bool { true }
    fn on_damage(&mut self, evt: &mut DamageEvent, ctx: &Ctx) -> bool { true }
    fn on_chat(&mut self, evt: &mut ChatEvent, ctx: &Ctx) -> bool { true }

    // Notification events (observe only)
    fn on_latency(&mut self, evt: &LatencySample, ctx: &Ctx) {}
    fn on_tick(&mut self, evt: &TickEvent, ctx: &Ctx) {}
    fn on_game_start(&mut self, evt: &GameStartEvent, ctx: &Ctx) {}
}
```

**No `on_move`** — poll `Player::position()` in `on_tick` instead. See
the example plugin for a teleport-detector implementation that uses
this pattern.

**Interceptable events** (return `bool`):
- Mutate `evt` fields freely before returning.
- Return `true` → forward call with modified params.
- Return `false` → skip original RPC entirely.
- Multi-plugin: load-order, first `false` short-circuits.

---

## Events

### PlayerJoinEvent

| Field | Type | Description |
|-------|------|-------------|
| `player` | `PlayerId` (u64) | Opaque player key |
| `name` | `String` | Name announced by Mirror |

Helpers: `evt.player(ctx) -> Player<'_>`

### DamageEvent

Source: `PlayerControl.RpcDamageEntities(int, int, int, int, int, string)`

| Field | Type | Description |
|-------|------|-------------|
| `attacker` | `PlayerId` | Who dealt damage (always set) |
| `victim` | `PlayerId` | Target (0 if NPC) |
| `amount` | `i32` | Damage value |
| `damage_type` | `i32` | See `DamageType` enum |
| `weapon_id` | `i32` | See `WeaponId` / `GadgetId` enums |
| `is_npc` | `bool` | Target is NPC |
| `data` | `String` | Extra RPC data |
| `frame` | `u64` | Frame counter (0 if unwired) |

Helpers:
- `evt.attacker(ctx) -> Player<'_>`
- `evt.victim(ctx) -> Option<Player<'_>>` (None if NPC)
- `evt.damage_type_enum() -> Option<DamageType>`
- `evt.weapon_enum() -> Option<WeaponId>`
- `evt.gadget_enum() -> Option<GadgetId>`

### ChatEvent (interceptable)

Source: `PlayerControl.UserCode_CmdSendChat__String(string)`

| Field | Type | Description |
|-------|------|-------------|
| `sender` | `PlayerId` | Who sent the message |
| `message` | `String` | Chat content (**mutable** — modify before Allow) |

Helpers: `evt.sender(ctx) -> Player<'_>`

Return `false` to swallow, `true` to forward (with possible modifications).

### LatencySample

| Field | Type | Description |
|-------|------|-------------|
| `player` | `PlayerId` | |
| `ms` | `f32` | Round-trip latency in ms |

### TickEvent

| Field | Type | Description |
|-------|------|-------------|
| `frame` | `u64` | Monotonic frame counter |

### GameStartEvent

Source: hooks `GameManager.Start`, emitted **after** the original runs.
Notification only — cannot be intercepted.

| Field | Type | Description |
|-------|------|-------------|
| `frame` | `u64` | Tick frame counter at emit time (`0` if tick collector not yet started) |

Use to reset per-game state. Query the new map / match type via
`ctx.game_map()` / `ctx.match_type()` — both are populated by the time
the event fires.

---

## Player handle

`Player<'ctx>` — constructed via `evt.player(ctx)`, `ctx.player(id)`, etc.

### Identity / network

| Method | Return | Field / source |
|--------|--------|----------------|
| `name()` | `String` | `_playerID` (editable display name) |
| `player_id()` | `u32` | slot N parsed from `unity_name()` `"Player<N>"`; `0` if not parseable |
| `device_id()` | `String` | `deviceID` |
| `team()` | `String` | `team` |
| `ground_type()` | `String` | `groundType` |

### Lifecycle / state

| Method | Return | Field |
|--------|--------|-------|
| `health()` | `i32` | `health` |
| `health_regen_cooldown()` | `f32` | `healthRegenCooldown` |
| `is_dead()` | `bool` | `dead` |
| `is_ready()` | `bool` | `ready` |
| `respawn_timer()` | `f32` | `respawnTimer` |
| `done_loading_map()` | `bool` | `doneLoadingMap` |
| `user_state()` | `i32` | `myState` (enum) |
| `class_role()` | `i32` | `myClass` (enum) |

### Stats / counters

| Method | Return | Field |
|--------|--------|-------|
| `kill_count()` | `i32` | `killCount` |
| `death_count()` | `i32` | `deathCount` |
| `bullets_fired()` | `i32` | `bulletsFired` |
| `grenades_thrown()` | `i32` | `grenadesThrown` |
| `reloads_done()` | `i32` | `reloadsDone` |
| `kill_rate()` | `i32` | `killRate` |
| `damage_rate()` | `i32` | `damageRate` |
| `network_rate()` | `i32` | `networkRate` |
| `latency_rate()` | `i32` | `latencyRate` |
| `ping_warn()` | `i32` | `pingWarn` |
| `teamkill_warn()` | `i32` | `teamKillWarn` |

### Movement / pose

| Method | Return | Field |
|--------|--------|-------|
| `speed()` | `f32` | `playerSpeed` |
| `is_running()` | `bool` | `running` |
| `is_grounded()` | `bool` | `grounded` |
| `crouch()` | `i32` | `crouch` |
| `is_under_water()` | `bool` | `isUnderWater` |
| `position()` | `[f32; 3]` | `_netTransform._recivedPos` (falls back to `lastPlayerPos`) |
| `velocity()` | `[f32; 3]` | `_netTransform._recivedVel` (falls back to `myRigidVel`) |
| `move_dir()` | `[f32; 3]` | `_moveDir` |
| `look_dir()` | `[f32; 2]` | `_lookDir` |

### Combat-adjacent

| Method | Return | Field |
|--------|--------|-------|
| `trying_to_attack()` | `f32` | `tryingToAttack` |
| `obstacle_timer()` | `f32` | `obstacleTimer` |
| `expose_timer()` | `f32` | `exposeTimer` |
| `dont_expose()` | `bool` | `dontExpose` |

### Input / camera

| Method | Return | Field |
|--------|--------|-------|
| `mouse_x()` | `f32` | `mouseX` |
| `mouse_y()` | `f32` | `mouseY` |
| `input_x()` | `f32` | `inputX` |
| `input_y()` | `f32` | `inputY` |
| `auto_sprint()` | `bool` | `autoSprint` |
| `head_bob()` | `bool` | `headBob` |
| `joystick_lean()` | `bool` | `joystickLean` |
| `cam_sensitivity()` | `f32` | `camSensitivity` |
| `ads_sensitivity()` | `f32` | `adsSensitivity` |
| `gyro_look_sensitivity()` | `f32` | `gyroLookSensitivity` |
| `gyro_ads_sensitivity()` | `f32` | `gyroAdsSensitivity` |
| `local_cam_dist()` | `f32` | `localCamDist` |
| `cam_fov()` | `f32` | `camFov` |
| `cam_shake()` | `f32` | `camShake` |
| `default_lod_bias()` | `f32` | `defaultLodBias` |

### Network / latency

| Method | Return | Field |
|--------|--------|-------|
| `latency()` | `f32` | `myLatency` |
| `ip()` | `String` | `connectionToClient.address` (Mirror) |
| `unity_name()` | `String` | `UnityEngine.Object.get_name()` (`"Player3"`) — slot integer is in `player_id()` |

### Combat

| Method | Return | Field |
|--------|--------|-------|
| `weapon_id()` | `i32` | `playerCombat.currWeaponID` |

### Voting

| Method | Return | Field |
|--------|--------|-------|
| `vote_kicked()` | `bool` | `voteKicked` |
| `voted()` | `bool` | `voted` |

### Actions (on Player)

All hardcoded against Polyfield's known C# methods — no config needed.

| Method | Return | Backed by |
|--------|--------|-----------|
| `is_host()` | `bool` | `PlayerControl.get_isLocalPlayer()` |
| `set_health(health, flag)` | `()` | `PlayerControl.RpcUpdateHealth(int, int)` |
| `show_error(title, body)` | `()` | `PlayerControl.RpcErrorPanel(string, string)` |
| `kill()` | `()` | `PlayerControl.RpcKillMe()` |
| `kick_me(delay_secs)` | `()` | `MonoBehaviour.Invoke("KickMePlz", float)` |
| `kick_with_reason(title, body, delay_secs)` | `()` | `show_error` + `kick_me` (convenience) |

---

## Game enums (`polyfield::game_enums`)

### DamageType

| Value | Variant | Name |
|-------|---------|------|
| 0 | `Accident` | accident |
| 1 | `Bullet` | bullet |
| 2 | `Launcher` | launcher |
| 3 | `Grenade` | grenade |
| 4 | `Shell` | shell |
| 5 | `VehicleExplosion` | vehicleExplosion |
| 6 | `Artillery` | artillery |
| 7 | `Nuke` | nuke |

### WeaponId

| Value | Variant | Name |
|-------|---------|------|
| 0 | `M1915` | M1915 |
| 1 | `M1Garand` | M1 Garand |
| 2 | `Mp40` | MP40 |
| 3 | `Kar98k` | Kar98k |
| 4 | `Stg44` | STG 44 |
| 5 | `Mg42` | MG42 |
| 6 | `Sten` | Sten |
| 7 | `Mg34` | MG34 |
| 8 | `M2Browning` | M2Browning |
| 9 | `Welrod` | Welrod |

### GadgetId

| Value | Variant | Name |
|-------|---------|------|
| 0 | `Bazooka` | Bazooka |
| 1 | `BandagePouch` | BandagePouch |
| 2 | `AmmoPouch` | AmmoPouch |
| 3 | `Panzerschreck` | Panzerschreck |

### MatchType

Backs `GameManager.matchType`. Variant names match the C# enum (camelCase)
so they round-trip with `ctx.match_type()`.

| Value | Variant | Name |
|-------|---------|------|
| 0 | `teamMatch` | teamMatch |
| 1 | `conquest` | conquest |

All enums provide:
- `from_raw(i32) -> Option<Self>`
- `name() -> &'static str`
- `impl Display`

---

## Interception (bool)

Interceptable events (`on_player_join`, `on_damage`, `on_chat`) return `bool`:

- `true` → proceed with the (possibly modified) call.
- `false` → cancel the call entirely.

The default impl returns `true`, so plugins that don't override an
interceptable event behave as pure observers.

Multi-plugin rule: plugins are called in load order; the first `false`
short-circuits subsequent plugins and skips the original game function.

---

## Ctx actions

| Method | Description |
|--------|-------------|
| `ctx.log_info(msg)` | Log at info level |
| `ctx.log_warn(msg)` | Log at warn level |
| `ctx.log_error(msg)` | Log at error level |
| `ctx.force_quit()` | Terminate game process |
| `ctx.show_dialog(title, body)` | Show in-game modal |
| `ctx.player(id) -> Player<'_>` | Get player handle |
| `ctx.player_info(id) -> PlayerInfo` | Snapshot common fields |
| `ctx.host_player() -> Option<Player<'_>>` | Host's player (`PlayerControl.get_Local()`) |
| `ctx.player_by_name(name) -> Option<Player<'_>>` | Find by name (`PlayersManager.GetPlayer`) |
| `ctx.player_by_id(id) -> Option<Player<'_>>` | Find by slot id (looks up `Player{id}`) |
| `ctx.players() -> Vec<PlayerSnapshot>` | Snapshot all players |
| `ctx.game_map() -> String` | Current map (`GameManager.Instance.GetMapName()`, `"-..."` suffix stripped) |
| `ctx.match_type() -> String` | Current match type (`"teamMatch"` / `"conquest"` / `"unknown:N"`) |
| `ctx.entities_inspect() -> String` | `ServerEntityInspector.Instance.GetAnalytics()` analytics dump |

For per-player actions (kick / ban / kill / set_health / show_error), use
the methods on `Player` (see [Actions on Player](#actions-on-player)).

---

## polyfield.toml configuration

```toml
plugins_dir = "plugins"
tick_interval_ms = 50    # on_tick cadence

[dump]
enabled = false
dir     = "dump"
mode    = "single"       # or "per_assembly"

# Host-level actions — both optional. If unset, force_quit falls back to
# exit(0) and show_dialog logs a warning and no-ops.

[actions.quit]
class  = "UnityEngine.Application"
method = "Quit"
args   = []

[actions.show_dialog]
class  = "UIManager"
method = "ShowModal"
args   = ["System.String", "System.String"]
```

Per-player actions — `kick_me`, `kick_with_reason`, `kill`, `set_health`,
`show_error` — are **hardcoded** against Polyfield's known C# methods
and don't need entries here. Call them on the [`Player`](#actions-on-player)
handle.
