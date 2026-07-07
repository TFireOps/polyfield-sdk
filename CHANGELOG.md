# Changelog

## 1.0.0 (2026-07-07) — ABI 31

First unified release: SDK crate version now tracks the unified
`polyfield-sdk` release line (framework `libpolyfield.so` + `polyfield-agent`
+ installer ship together under one `vX.Y.Z` tag).

Sync from framework, ABI 22 → 31. Highlights:

### Timers
- `Ctx::schedule_once(delay_ms, token)` one-shot timers, delivered via the new
  `on_timer(token)` plugin callback. Host owns scheduling; plugins match on
  `token`. No cancellation yet — keep tokens meaningful.

### Plugin config & data directories
- `Ctx::read_config()` / `config_path()` / `config_dir()` — per-plugin
  `config/<plugin>.toml` convention, host-managed paths.
- `Ctx::data_dir()` for plugin-private persistent data.

### Server control & introspection
- Map rotation: `Ctx::server_maps()` / `next_map()` / `set_next_map()`.
- Server info: `server_port()` / `server_region()` / `server_link()` /
  `max_players()` / `player_count()` / `game_version()`.

### Game tuning
- `Ctx::damage_factor()` and `explosion_damage()` read access to global
  damage tuning.

### Key-value store
- `kv_del()` / `kv_del_global()` / `kv_clear_prefix()` — deletion and
  prefix-clearing for the local and cross-plugin global KV store.

### Player API
- Accounts: `Player::account_id()` / `is_logged_in()` (panel-backed login).
- Control: `freeze()` / `set_velocity()`.
- Presentation: `set_corner_text()`, `set_latency_display()` /
  `broadcast_latency_display()` (host-side latency display control,
  `player_update_latency` at the ABI level).

### Expanded game enums
- `game_enums` grew substantially: weapon models (Kar98k, M1Garand, Bazooka,
  MG34, …), soldier classes (Assault / Medic / …), vehicle kinds
  (Jeep / Airplane / Artillery, …), network roles (Host / Client / Admin),
  leave reasons (Kicked / Banned / Accident), pickup/gadget types, and more.

### Docs
- Bilingual doc comments (EN + zh-CN) across the public API surface.

## 0.3.0 (2026-06-11) — ABI 22

- True private chat via `SendTargetRPCInternal`.
- SDK v21→v22 sync: hardened and expanded plugin API surface.
