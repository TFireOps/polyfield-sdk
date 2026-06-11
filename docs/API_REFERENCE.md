# API Reference

Complete reference for the Polyfield plugin SDK.

---

## Plugin trait

```rust
pub trait Plugin: Send + Sync + 'static {
    fn manifest() -> &'static PluginManifest where Self: Sized;

    // Notification (observe only, no return)
    fn on_load(&mut self, ctx: &Ctx) {}
    fn on_unload(&mut self, ctx: &Ctx) {}
    fn on_player_join(&mut self, evt: &PlayerJoinEvent, ctx: &Ctx) {}
    fn on_latency(&mut self, evt: &LatencySample, ctx: &Ctx) {}
    fn on_tick(&mut self, evt: &TickEvent, ctx: &Ctx) {}
    fn on_game_start(&mut self, evt: &GameStartEvent, ctx: &Ctx) {}
    fn on_reload(&mut self, evt: &ReloadEvent, ctx: &Ctx) {}

    // Interceptable (mutate evt; return false to skip the original RPC)
    fn on_damage(&mut self, evt: &mut DamageEvent, ctx: &Ctx) -> bool { true }
    fn on_chat(&mut self, evt: &mut ChatEvent, ctx: &Ctx) -> bool { true }
    fn on_respawn(&mut self, evt: &mut RespawnEvent, ctx: &Ctx) -> bool { true }
    fn on_grenade(&mut self, evt: &mut GrenadeEvent, ctx: &Ctx) -> bool { true }
    fn on_shoot(&mut self, evt: &mut ShootEvent, ctx: &Ctx) -> bool { true }
    fn on_vehicle_shoot(&mut self, evt: &mut VehicleShootEvent, ctx: &Ctx) -> bool { true }
    fn on_vehicle_repair(&mut self, evt: &mut VehicleRepairEvent, ctx: &Ctx) -> bool { true }

    // Backend channel (v22)
    fn on_command(&mut self, name: &str, args: &str, ctx: &Ctx) -> Option<String> { None }
    fn on_timer(&mut self, token: u64, ctx: &Ctx) {}
}
```

A panic inside any callback is caught at the ABI boundary, logged, and
**fails open** (interceptable events forward the original call) — a buggy
plugin never crashes the game process.

**`on_command`** — inbound command routed from the management backend.
Return `Some(reply)` to answer, `None` for no reply. Pairs with
[`Ctx::emit`](#ctx-handle) for the outbound direction.

**`on_timer`** — a one-shot armed via `Ctx::schedule_once(delay_ms, token)`
has elapsed; `token` is the value you passed. Use it for deferred actions a
single `on_tick` cadence can't express ("warn now, re-check in 3s").

---

## Events

### PlayerJoinEvent
`player: PlayerRef`, `name: String`. Notification only — fires on first sighting / rename (deduped).

### DamageEvent
`attacker: PlayerRef`, `victim_slot: u32`, `amount: i32`, `damage_type: i32`, `weapon_id: i32`, `is_npc: bool`, `data: String`, `frame: u64`
Helpers: `attacker(ctx) -> Player`, `victim(ctx) -> Option<Player>` (resolves the slot via `PlayersManager.GetPlayer`), `damage_type_enum()`, `weapon() -> Option<WeaponId>`, `gadget_enum()`

### ChatEvent
`sender: PlayerRef`, `message: String` (mutable)

### RespawnEvent
`player: PlayerRef`, `spawn_data: String` (mutable), `vehicle_type: u32`

### GrenadeEvent
`player: PlayerRef`, `grenade_data: String` (mutable)

### ShootEvent
`player: PlayerRef`, `weapon_type: u8`, `shoot_data: String` (mutable). Helper: `weapon() -> Option<WeaponId>`

### VehicleShootEvent
`player: PlayerRef`, `vehicle: PlayerRef`, `vehicle_id: u32`, `seat_id: i32`

### VehicleRepairEvent
`player: PlayerRef`, `vehicle_id: u32`, `timer: i32`, `health: i32`

### ReloadEvent
`player: PlayerRef`, `anim_name: String`

### LatencySample
`player: PlayerRef`, `ms: f32`

### TickEvent / GameStartEvent
`frame: u64`

---

## Player handle

### Identity
`name() -> String`, `player_id() -> u32`, `unity_name() -> String`, `device_id() -> String`, `team() -> String`, `ip() -> String`, `is_host() -> bool`

### Lifecycle
`health() -> i32`, `health_regen_cooldown() -> f32`, `is_dead() -> bool`, `is_ready() -> bool`, `respawn_timer() -> f32`, `done_loading_map() -> bool`, `user_state() -> Option<UserState>`, `user_state_raw() -> i32`, `class_role() -> Option<ClassRole>`, `class_role_raw() -> i32`

### Stats
`kill_count()`, `death_count()`, `bullets_fired()`, `grenades_thrown()`, `reloads_done()`, `kill_rate()`, `damage_rate()`, `network_rate()`, `latency_rate()`, `ping_warn()`, `teamkill_warn()` — all `i32`; `kdr() -> f32` (derived)

### Movement
`position() -> [f32; 3]` (net pos, falls back to lastPlayerPos), `net_position() -> Option<[f32; 3]>` (no fallback; `None` if not yet replicated), `velocity() -> [f32; 3]`, `net_velocity() -> Option<[f32; 3]>`, `speed() -> f32` (⚠️ configured move-speed **constant**, not live velocity — use `velocity()` magnitude for speed checks), `move_dir() -> [f32; 3]`, `look_dir() -> [f32; 2]`, `is_running() -> bool`, `is_grounded() -> bool`, `crouch() -> i32`, `is_under_water() -> bool`
`pos() -> Vec3`, `vel() -> Vec3` — same as `position()`/`velocity()` wrapped in [`Vec3`](#vec3) for distance math.

### Combat
`weapon() -> Option<WeaponId>`, `weapon_id() -> i32`, `trying_to_attack() -> f32`, `obstacle_timer() -> f32`, `expose_timer() -> f32`, `dont_expose() -> bool`

### Input / Camera
`mouse_x()`, `mouse_y()`, `input_x()`, `input_y()`, `cam_sensitivity()`, `ads_sensitivity()`, `cam_fov()`, `local_cam_dist()` — all `f32`

### Network
`latency() -> f32`, `ip() -> String`

### Voting
`voted() -> bool`, `vote_kicked() -> bool`

### Actions
`kill()`, `kick_me(delay: f32)`, `kick_with_reason(title, body, delay)`, `set_health(hp: i32, flag: i32)`, `show_error(title, body)` — `kill()`/`kick_me()` are no-ops (logged) when the handle is the host player.
`send_chat_to(msg)` (directed chat to this player's client only), `update_name(name)` (force display name via `RpcUpdateName`), `call_animation(anim)` (trigger animation via `RpcCallAnimation`)

### Vehicle association
`vehicle() -> Option<Vehicle>` (the vehicle this player is in, `None` on foot), `is_in_vehicle() -> bool` — the inverse of `Vehicle::driver()`.

### Raw field escape hatch
`read_raw_i32(field)`, `read_raw_f32(field)`, `read_raw_bool(field)`, `read_raw_vec3(field)` — read any `PlayerControl` field by its `polyfield::fields::F_*` id when there's no dedicated typed getter yet. The host still validates the id against its offset table (unknown → default).

---

## Vehicle handle

`health() -> i32`, `vehicle_type() -> Option<VehicleType>`, `vehicle_type_raw() -> i32`, `model_name() -> String` (specific model string, e.g. `"jagdpanther"`), `position() -> [f32; 3]`, `rotation() -> [f32; 3]`, `velocity() -> [f32; 3]`, `driver() -> Option<Player>`, `driver_ref() -> PlayerRef`, `raw() -> PlayerRef`

---

## Ctx handle

| Method | Description |
|---|---|
| `log_info/warn/error(msg)` | Write to polyfield.log |
| `force_quit()` | Terminate game |
| `show_dialog(title, body)` | In-game modal |
| `host_say(msg)` | Chat as host (auto-prefixes "(raw)") |
| `player(id) -> Player` | Build player handle |
| `host_player() -> Option<Player>` | Host player |
| `player_by_name(name) -> Option<Player>` | Find by name |
| `player_by_id(slot) -> Option<Player>` | Find by slot |
| `players() -> Vec<PlayerSnapshot>` | All online players (read-only snapshots) |
| `all_players() -> Vec<Player>` | All online players as **live handles** (for server-wide scans) |
| `vehicle(id) -> Vehicle` | Build vehicle handle |
| `vehicles() -> Vec<Vehicle>` | All active vehicles |
| `game_map() -> String` | Current map |
| `match_type() -> String` | Current match type (string) |
| `match_type_enum() -> Option<MatchType>` | Current match type (typed) |
| `current_time() -> Option<f32>` | Match countdown (seconds); `None` if no GameManager |
| `set_current_time(secs)` | Force the countdown (e.g. `10.0` to trigger map rotation) |
| `emit(kind, json)` | Send a structured event to the management backend |
| `schedule_once(delay_ms, token)` | Arm a one-shot timer → fires `on_timer(token)` |
| `kv_set(key, value)` | KV write, auto-namespaced by plugin |
| `kv_get(key) -> Option<String>` | KV read of this plugin's namespace |
| `kv_set_global(key, value)` | KV write, shared across plugins |
| `kv_get_global(key) -> Option<String>` | KV read, shared across plugins |

`players()` vs `all_players()`: snapshots are a fixed, immutable field set
(id/pos/velocity/health/latency) cheap to collect in bulk; live handles read
current state on demand and expose the full accessor + action surface — use
them for speed checks, load-gating, per-team messaging, kd tallies.

---

## Game enums

**DamageType**: Accident(0), Bullet(1), Launcher(2), Grenade(3), Shell(4), VehicleExplosion(5), Artillery(6), Nuke(7)

**WeaponId**: M1915(0), M1Garand(1), Mp40(2), Kar98k(3), Stg44(4), Mg42(5), Sten(6), Mg34(7), M2Browning(8), Welrod(9)

**GadgetId**: Bazooka(0), BandagePouch(1), AmmoPouch(2), Panzerschreck(3)

**VehicleType**: None(0), Jeep(1), Tank(2), Airplane(3)

**MatchType**: teamMatch(0), conquest(1) — also `from_name(&str) -> Option<Self>`

**UserState**: client(0), host(1), admin(2), kicked(3), banned(4) — also `is_privileged() -> bool`

**ClassRole**: assault(0), medic(1), support(2), scout(3), tanker(4)

All enums: `from_raw(i32) -> Option<Self>`, `name() -> &str`, `Display`

---

## Utility

```rust
polyfield::color(color: &str, msg: impl Display) -> String
```

Wraps in `<color=...>...</color>` for in-game rich text.

### Vec3

```rust
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }
```

3-component vector for distance math. Convertible to/from `[f32; 3]`
(`From`/`Into`), so `player.position().into()` works, or use `player.pos()`.

| Method | Description |
|---|---|
| `Vec3::new(x, y, z)` | Construct |
| `distance(other) -> f32` | 3D Euclidean distance |
| `distance_sq(other) -> f32` | Squared 3D distance (skip sqrt for threshold compares) |
| `distance_2d(other) -> f32` | Horizontal (XZ) distance, ignores Y — right metric for ground movement |
| `magnitude() -> f32` | Length from origin |
| `magnitude_2d() -> f32` | Horizontal speed (apply to a velocity vector) |
| `to_array() -> [f32; 3]` | Underlying array |
