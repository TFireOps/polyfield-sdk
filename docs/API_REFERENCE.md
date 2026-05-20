# API Reference

Complete reference for the Polyfield plugin SDK.

---

## Plugin trait

```rust
pub trait Plugin: Send + Sync + 'static {
    fn manifest() -> &'static PluginManifest where Self: Sized;
    fn on_load(&mut self, ctx: &Ctx) {}
    fn on_unload(&mut self, ctx: &Ctx) {}
    fn on_player_join(&mut self, evt: &mut PlayerJoinEvent, ctx: &Ctx) -> bool { true }
    fn on_damage(&mut self, evt: &mut DamageEvent, ctx: &Ctx) -> bool { true }
    fn on_chat(&mut self, evt: &mut ChatEvent, ctx: &Ctx) -> bool { true }
    fn on_respawn(&mut self, evt: &mut RespawnEvent, ctx: &Ctx) -> bool { true }
    fn on_grenade(&mut self, evt: &mut GrenadeEvent, ctx: &Ctx) -> bool { true }
    fn on_shoot(&mut self, evt: &mut ShootEvent, ctx: &Ctx) -> bool { true }
    fn on_vehicle_shoot(&mut self, evt: &mut VehicleShootEvent, ctx: &Ctx) -> bool { true }
    fn on_vehicle_repair(&mut self, evt: &mut VehicleRepairEvent, ctx: &Ctx) -> bool { true }
    fn on_latency(&mut self, evt: &LatencySample, ctx: &Ctx) {}
    fn on_tick(&mut self, evt: &TickEvent, ctx: &Ctx) {}
    fn on_game_start(&mut self, evt: &GameStartEvent, ctx: &Ctx) {}
    fn on_reload(&mut self, evt: &ReloadEvent, ctx: &Ctx) {}
}
```

---

## Events

### PlayerJoinEvent
`player: PlayerRef`, `name: String`

### DamageEvent
`attacker: PlayerRef`, `victim: PlayerRef` (netId), `amount: i32`, `damage_type: i32`, `weapon_id: i32`, `is_npc: bool`, `data: String`, `frame: u64`
Helpers: `damage_type_enum()`, `weapon_enum()`, `gadget_enum()`

### ChatEvent
`sender: PlayerRef`, `message: String` (mutable)

### RespawnEvent
`player: PlayerRef`, `spawn_data: String` (mutable), `vehicle_type: u32`

### GrenadeEvent
`player: PlayerRef`, `grenade_data: String` (mutable)

### ShootEvent
`player: PlayerRef`, `weapon_type: u8`, `shoot_data: String` (mutable). Helper: `weapon_enum()`

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
`health() -> i32`, `health_regen_cooldown() -> f32`, `is_dead() -> bool`, `is_ready() -> bool`, `respawn_timer() -> f32`, `done_loading_map() -> bool`, `user_state() -> i32`, `class_role() -> i32`

### Stats
`kill_count()`, `death_count()`, `bullets_fired()`, `grenades_thrown()`, `reloads_done()`, `kill_rate()`, `damage_rate()`, `network_rate()`, `latency_rate()`, `ping_warn()`, `teamkill_warn()` — all `i32`

### Movement
`position() -> [f32; 3]`, `velocity() -> [f32; 3]`, `move_dir() -> [f32; 3]`, `look_dir() -> [f32; 2]`, `is_running() -> bool`, `is_grounded() -> bool`, `crouch() -> i32`, `is_under_water() -> bool`

### Combat
`weapon_id() -> i32`, `trying_to_attack() -> f32`, `obstacle_timer() -> f32`, `expose_timer() -> f32`, `dont_expose() -> bool`

### Input / Camera
`mouse_x()`, `mouse_y()`, `input_x()`, `input_y()`, `cam_sensitivity()`, `ads_sensitivity()`, `cam_fov()`, `local_cam_dist()` — all `f32`

### Network
`latency() -> f32`, `ip() -> String`

### Voting
`voted() -> bool`, `vote_kicked() -> bool`

### Actions
`kill()`, `kick_me(delay: f32)`, `kick_with_reason(title, body, delay)`, `set_health(hp: i32, flag: i32)`, `show_error(title, body)`

---

## Vehicle handle

`health() -> i32`, `vehicle_type() -> Option<VehicleType>`, `vehicle_type_raw() -> i32`, `position() -> [f32; 3]`, `rotation() -> [f32; 3]`, `velocity() -> [f32; 3]`, `driver() -> Option<Player>`, `driver_ref() -> PlayerRef`, `raw() -> PlayerRef`

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
| `players() -> Vec<PlayerSnapshot>` | All online players |
| `vehicle(id) -> Vehicle` | Build vehicle handle |
| `vehicles() -> Vec<Vehicle>` | All active vehicles |
| `game_map() -> String` | Current map |
| `match_type() -> String` | Current match type |
| `kv_set(key, value)` | Shared KV write |
| `kv_get(key) -> Option<String>` | Shared KV read |

---

## Game enums

**DamageType**: Accident(0), Bullet(1), Launcher(2), Grenade(3), Shell(4), VehicleExplosion(5), Artillery(6), Nuke(7)

**WeaponId**: M1915(0), M1Garand(1), Mp40(2), Kar98k(3), Stg44(4), Mg42(5), Sten(6), Mg34(7), M2Browning(8), Welrod(9)

**GadgetId**: Bazooka(0), BandagePouch(1), AmmoPouch(2), Panzerschreck(3)

**VehicleType**: None(0), Jeep(1), Tank(2), Airplane(3)

**MatchType**: teamMatch(0), conquest(1)

All enums: `from_raw(i32) -> Option<Self>`, `name() -> &str`, `Display`

---

## Utility

```rust
polyfield::color(color: &str, msg: impl Display) -> String
```

Wraps in `<color=...>...</color>` for in-game rich text.
