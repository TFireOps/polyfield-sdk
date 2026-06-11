//! Stable C ABI between the framework (host) and plugins.
//!
//! Plugin authors never touch this module directly. `declare_plugin!`
//! generates the `extern "C"` entry that returns a [`PluginVTable`]; the
//! framework looks up that entry via `dlsym` and drives the plugin through
//! the vtable for its lifetime.

use crate::context::{Ctx, PlayerSnapshot};
use crate::events::{
    ChatEvent, DamageEvent, GameStartEvent, GrenadeEvent, LatencySample, PlayerJoinEvent,
    PlayerRef, ReloadEvent, RespawnEvent, ShootEvent, TickEvent, VehicleRepairEvent,
    VehicleShootEvent,
};
use crate::fields::PlayerField;
use crate::plugin::{Plugin, PluginManifest};
use std::ffi::{c_char, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Bump when the vtable or any event layout changes in a way that requires
/// recompiling plugins. The framework refuses to load plugins whose
/// manifest reports a different value.
pub const POLYFIELD_ABI_VERSION: u32 = 21;

pub const POLYFIELD_ENTRY_SYMBOL: &[u8] = b"polyfield_plugin_entry\0";

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Info = 0,
    Warn = 1,
    Error = 2,
}

/// ABI-internal. Exposed only because [`crate::declare_plugin!`] expands
/// to a function that names it. Plugin authors never construct or call
/// through this directly — use the [`crate::Ctx`] / [`crate::Player`]
/// wrappers, which uphold the safety invariants this raw vtable does not.
///
/// ABI 内部类型，仅因 [`crate::declare_plugin!`] 展开会用到它而公开。
/// 插件作者不应直接构造或调用——请用 [`crate::Ctx`] / [`crate::Player`]
/// 封装，它们维护了这个裸 vtable 不保证的安全不变量。
#[doc(hidden)]
#[repr(C)]
pub struct HostApi {
    pub log: unsafe extern "C" fn(LogLevel, plugin: *const c_char, msg: *const c_char),
    pub force_quit: unsafe extern "C" fn(),
    pub show_dialog: unsafe extern "C" fn(title: *const c_char, body: *const c_char),
    pub players: unsafe extern "C" fn(out: *mut *const PlayerSnapshot, len: *mut usize),

    // Generic per-field readers. Adding a new field on PlayerControl
    // does not require extending this vtable — register the offset
    // server-side and expose a typed getter on `Player`.
    //
    // 通用按字段读取器。新增 PlayerControl 字段无需扩展该 vtable，
    // 只要在宿主侧登记偏移并在 `Player` 上暴露带类型的 getter 即可。
    pub player_read_i32: unsafe extern "C" fn(PlayerRef, PlayerField) -> i32,
    pub player_read_f32: unsafe extern "C" fn(PlayerRef, PlayerField) -> f32,
    pub player_read_bool: unsafe extern "C" fn(PlayerRef, PlayerField) -> bool,
    pub player_read_vec2: unsafe extern "C" fn(PlayerRef, PlayerField, out_xy: *mut f32),
    pub player_read_vec3: unsafe extern "C" fn(PlayerRef, PlayerField, out_xyz: *mut f32),
    /// Same `out_buf, cap -> required` convention used elsewhere; see
    /// [`crate::Player::name`] for an example caller.
    pub player_read_string:
        unsafe extern "C" fn(PlayerRef, PlayerField, out_buf: *mut c_char, cap: usize) -> usize,

    // Game-specific per-player actions. These call hardcoded C# methods on
    // PlayerControl / MonoBehaviour and don't depend on `polyfield.toml`.
    //
    // 游戏特定的单玩家动作。直接调用 PlayerControl / MonoBehaviour 上
    // 硬编码的 C# 方法，不依赖 `polyfield.toml`。
    pub player_is_host: unsafe extern "C" fn(PlayerRef) -> bool,
    pub player_set_health: unsafe extern "C" fn(PlayerRef, health: i32, flag: i32),
    pub player_show_error: unsafe extern "C" fn(PlayerRef, title: *const c_char, body: *const c_char),
    pub player_kill: unsafe extern "C" fn(PlayerRef),
    pub player_kick_me: unsafe extern "C" fn(PlayerRef, delay_secs: f32),
    /// Reads the player's network IP via Mirror's connection chain.
    /// `out_buf, cap -> required` convention.
    pub player_ip:
        unsafe extern "C" fn(PlayerRef, out_buf: *mut c_char, cap: usize) -> usize,
    /// Reads the player's GameObject name (slot identifier like "Player3").
    /// `out_buf, cap -> required` convention.
    pub player_unity_name:
        unsafe extern "C" fn(PlayerRef, out_buf: *mut c_char, cap: usize) -> usize,

    /// Returns the host's `PlayerControl` instance ref, or `0` if not available.
    pub host_ref: unsafe extern "C" fn() -> PlayerRef,

    /// Look up a player by their internal name string. Returns `0` when not found.
    pub find_player: unsafe extern "C" fn(name: *const c_char) -> PlayerRef,

    // Game-level singleton accessors. Hardcoded against Polyfield's known
    // C# methods (GameManager, ServerEntityInspector). All three use the
    // `out_buf, cap -> required` convention; an empty string means the
    // singleton or method couldn't be resolved.
    //
    // 游戏级单例访问器。直接调用 Polyfield 已知的 C# 方法
    // （GameManager、ServerEntityInspector）。三者都使用
    // `out_buf, cap -> required` 约定；返回空串表示单例或方法未解析。

    /// `GameManager.Instance.GetMapName()` with the `"-"` suffix stripped.
    pub game_map: unsafe extern "C" fn(out_buf: *mut c_char, cap: usize) -> usize,

    /// `GameManager.Instance.matchType` rendered as a string
    /// (`"teamMatch"` / `"conquest"` / `"unknown:N"`). Pair with
    /// [`crate::game_enums::MatchType`] for typed access.
    pub match_type: unsafe extern "C" fn(out_buf: *mut c_char, cap: usize) -> usize,

    /// `ServerEntityInspector.Instance.GetAnalytics()` — game-supplied
    /// JSON-ish dump. Cheap to call; large payload — copy and parse if
    /// you want to inspect it.
    pub entities_inspect: unsafe extern "C" fn(out_buf: *mut c_char, cap: usize) -> usize,

    /// Send a chat message as the host (`PlayerControl.CmdSendChat`).
    /// No-op if the host isn't available yet.
    pub host_say: unsafe extern "C" fn(msg: *const c_char),

    /// Store a value in the shared KV store. Key and value are both
    /// null-terminated C strings. Overwrites any existing value for the key.
    pub kv_set: unsafe extern "C" fn(key: *const c_char, value: *const c_char),

    /// Read a value from the shared KV store. Returns the number of bytes
    /// written (excluding NUL). If `cap == 0` or the key doesn't exist,
    /// returns 0. Uses the same `out_buf, cap -> len` convention as other
    /// string-returning APIs.
    pub kv_get: unsafe extern "C" fn(key: *const c_char, out_buf: *mut c_char, cap: usize) -> usize,

    // ── Vehicle field readers ──────────────────────────────────────────────
    /// Read `VehicleControl.health` (i32).
    pub vehicle_health: unsafe extern "C" fn(vehicle: PlayerRef) -> i32,
    /// Read `VehicleControl.vehicleType` (i32 enum).
    pub vehicle_type: unsafe extern "C" fn(vehicle: PlayerRef) -> i32,
    /// Read `VehicleControl.myDriver` -> PlayerControl ptr as PlayerRef.
    pub vehicle_driver: unsafe extern "C" fn(vehicle: PlayerRef) -> PlayerRef,
    /// Read `VehicleControl.recivedPos` -> [f32; 3].
    pub vehicle_position: unsafe extern "C" fn(vehicle: PlayerRef, out: *mut f32),
    /// Read `VehicleControl.recivedRot` -> [f32; 3].
    pub vehicle_rotation: unsafe extern "C" fn(vehicle: PlayerRef, out: *mut f32),
    /// Read `VehicleControl.recivedVel` -> [f32; 3].
    pub vehicle_velocity: unsafe extern "C" fn(vehicle: PlayerRef, out: *mut f32),

    /// Get all online vehicle VehicleControl pointers.
    pub vehicles: unsafe extern "C" fn(out: *mut *const PlayerRef, len: *mut usize),
}

#[doc(hidden)]
#[repr(C)]
pub struct PluginVTable {
    pub manifest: &'static PluginManifest,
    pub state: *mut (),
    pub on_load: unsafe extern "C" fn(*mut (), &'static HostApi),
    pub on_unload: unsafe extern "C" fn(*mut (), &'static HostApi),
    /// Notification — fires on first sighting / rename. Not interceptable
    /// (the name broadcast can't be blocked).
    pub on_player_join: unsafe extern "C" fn(*mut (), *const PlayerJoinEvent, &'static HostApi),
    /// Interceptable: returns `true` to forward, `false` to skip the original RPC.
    pub on_damage: unsafe extern "C" fn(*mut (), *mut DamageEvent, &'static HostApi) -> bool,
    pub on_latency: unsafe extern "C" fn(*mut (), *const LatencySample, &'static HostApi),
    pub on_tick: unsafe extern "C" fn(*mut (), *const TickEvent, &'static HostApi),
    /// Interceptable: returns `true` to forward, `false` to skip the original RPC.
    pub on_chat: unsafe extern "C" fn(*mut (), *mut ChatEvent, &'static HostApi) -> bool,
    /// Notification — fires after `GameManager.Start` runs. Not interceptable
    /// (blocking it would prevent the new game from initialising).
    pub on_game_start: unsafe extern "C" fn(*mut (), *const GameStartEvent, &'static HostApi),
    /// Interceptable: returns `true` to forward, `false` to skip the original RPC.
    pub on_respawn:
        unsafe extern "C" fn(*mut (), *mut RespawnEvent, &'static HostApi) -> bool,
    /// Interceptable: returns `true` to forward, `false` to skip the original RPC.
    pub on_grenade:
        unsafe extern "C" fn(*mut (), *mut GrenadeEvent, &'static HostApi) -> bool,
    /// Interceptable: returns `true` to forward, `false` to skip the original RPC.
    pub on_shoot: unsafe extern "C" fn(*mut (), *mut ShootEvent, &'static HostApi) -> bool,
    /// Notification — fires when a player starts reloading.
    pub on_reload: unsafe extern "C" fn(*mut (), *const ReloadEvent, &'static HostApi),
    /// Interceptable: returns `true` to forward, `false` to skip.
    pub on_vehicle_shoot:
        unsafe extern "C" fn(*mut (), *mut VehicleShootEvent, &'static HostApi) -> bool,
    /// Interceptable: returns `true` to forward, `false` to skip.
    pub on_vehicle_repair:
        unsafe extern "C" fn(*mut (), *mut VehicleRepairEvent, &'static HostApi) -> bool,
    pub drop: unsafe extern "C" fn(*mut ()),
}

// SAFETY: `state` is an owned Box-of-PluginCell produced by `__build_vtable`.
// It is only accessed through the vtable's `on_*` trampolines, which invoke
// the underlying `Plugin` impl — and `Plugin: Send + Sync` is required by
// the trait. The framework additionally guards concurrent mutation behind a
// RwLock. Together these satisfy the invariants `Send + Sync` expects.
unsafe impl Send for PluginVTable {}
unsafe impl Sync for PluginVTable {}

struct Cell {
    plugin: Box<dyn Plugin>,
    name: &'static str,
}

/// Hidden entry used by the `declare_plugin!` macro. Requires the concrete
/// plugin type so we can read the manifest through its `Plugin::manifest()`.
#[doc(hidden)]
pub fn __build_vtable<P: Plugin>(plugin: P) -> PluginVTable {
    let manifest = P::manifest();
    let cell = Box::new(Cell {
        plugin: Box::new(plugin),
        name: manifest.name,
    });
    PluginVTable {
        manifest,
        state: Box::into_raw(cell) as *mut (),
        on_load,
        on_unload,
        on_player_join,
        on_damage,
        on_latency,
        on_tick,
        on_chat,
        on_game_start,
        on_respawn,
        on_grenade,
        on_shoot,
        on_reload,
        on_vehicle_shoot,
        on_vehicle_repair,
        drop: drop_cell,
    }
}

#[inline(always)]
unsafe fn with_cell<'a>(state: *mut ()) -> &'a mut Cell {
    &mut *(state as *mut Cell)
}

/// Report a caught plugin panic through the host logger. Best-effort: if
/// the message itself can't be turned into a C string we just drop it.
/// Crucially this never re-panics, so the FFI boundary stays sound.
unsafe fn report_panic(host: &HostApi, plugin: &str, hook: &str) {
    let p = CString::new(plugin).unwrap_or_default();
    let m = CString::new(format!("panicked in {hook} (caught — not propagated)"))
        .unwrap_or_default();
    (host.log)(LogLevel::Error, p.as_ptr(), m.as_ptr());
}

/// Run a notification-style hook body, swallowing any panic. A panicking
/// plugin must never unwind across this `extern "C"` boundary (UB) nor
/// take down the game process.
#[inline]
unsafe fn guard_notify(host: &'static HostApi, name: &str, hook: &str, body: impl FnOnce()) {
    if catch_unwind(AssertUnwindSafe(body)).is_err() {
        report_panic(host, name, hook);
    }
}

/// Run an interceptable hook body. On panic we log and **fail open**
/// (return `true` = forward the original call) — blocking a game call
/// because a plugin crashed would be worse than letting it through.
#[inline]
unsafe fn guard_intercept(
    host: &'static HostApi,
    name: &str,
    hook: &str,
    body: impl FnOnce() -> bool,
) -> bool {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(v) => v,
        Err(_) => {
            report_panic(host, name, hook);
            true
        }
    }
}

unsafe extern "C" fn on_load(state: *mut (), host: &'static HostApi) {
    let c = with_cell(state);
    guard_notify(host, c.name, "on_load", || {
        c.plugin.on_load(&Ctx::new(host, c.name))
    });
}
unsafe extern "C" fn on_unload(state: *mut (), host: &'static HostApi) {
    let c = with_cell(state);
    guard_notify(host, c.name, "on_unload", || {
        c.plugin.on_unload(&Ctx::new(host, c.name))
    });
}
unsafe extern "C" fn on_player_join(
    state: *mut (),
    evt: *const PlayerJoinEvent,
    host: &'static HostApi,
) {
    let c = with_cell(state);
    guard_notify(host, c.name, "on_player_join", || {
        c.plugin.on_player_join(&*evt, &Ctx::new(host, c.name))
    });
}
unsafe extern "C" fn on_damage(state: *mut (), evt: *mut DamageEvent, host: &'static HostApi) -> bool {
    let c = with_cell(state);
    guard_intercept(host, c.name, "on_damage", || {
        c.plugin.on_damage(&mut *evt, &Ctx::new(host, c.name))
    })
}
unsafe extern "C" fn on_latency(state: *mut (), evt: *const LatencySample, host: &'static HostApi) {
    let c = with_cell(state);
    guard_notify(host, c.name, "on_latency", || {
        c.plugin.on_latency(&*evt, &Ctx::new(host, c.name))
    });
}
unsafe extern "C" fn on_tick(state: *mut (), evt: *const TickEvent, host: &'static HostApi) {
    let c = with_cell(state);
    guard_notify(host, c.name, "on_tick", || {
        c.plugin.on_tick(&*evt, &Ctx::new(host, c.name))
    });
}
unsafe extern "C" fn on_chat(state: *mut (), evt: *mut ChatEvent, host: &'static HostApi) -> bool {
    let c = with_cell(state);
    guard_intercept(host, c.name, "on_chat", || {
        c.plugin.on_chat(&mut *evt, &Ctx::new(host, c.name))
    })
}
unsafe extern "C" fn on_game_start(state: *mut (), evt: *const GameStartEvent, host: &'static HostApi) {
    let c = with_cell(state);
    guard_notify(host, c.name, "on_game_start", || {
        c.plugin.on_game_start(&*evt, &Ctx::new(host, c.name))
    });
}
unsafe extern "C" fn on_respawn(
    state: *mut (),
    evt: *mut RespawnEvent,
    host: &'static HostApi,
) -> bool {
    let c = with_cell(state);
    guard_intercept(host, c.name, "on_respawn", || {
        c.plugin.on_respawn(&mut *evt, &Ctx::new(host, c.name))
    })
}
unsafe extern "C" fn on_grenade(
    state: *mut (),
    evt: *mut GrenadeEvent,
    host: &'static HostApi,
) -> bool {
    let c = with_cell(state);
    guard_intercept(host, c.name, "on_grenade", || {
        c.plugin.on_grenade(&mut *evt, &Ctx::new(host, c.name))
    })
}
unsafe extern "C" fn on_shoot(
    state: *mut (),
    evt: *mut ShootEvent,
    host: &'static HostApi,
) -> bool {
    let c = with_cell(state);
    guard_intercept(host, c.name, "on_shoot", || {
        c.plugin.on_shoot(&mut *evt, &Ctx::new(host, c.name))
    })
}
unsafe extern "C" fn on_reload(state: *mut (), evt: *const ReloadEvent, host: &'static HostApi) {
    let c = with_cell(state);
    guard_notify(host, c.name, "on_reload", || {
        c.plugin.on_reload(&*evt, &Ctx::new(host, c.name))
    });
}
unsafe extern "C" fn on_vehicle_shoot(
    state: *mut (),
    evt: *mut VehicleShootEvent,
    host: &'static HostApi,
) -> bool {
    let c = with_cell(state);
    guard_intercept(host, c.name, "on_vehicle_shoot", || {
        c.plugin.on_vehicle_shoot(&mut *evt, &Ctx::new(host, c.name))
    })
}
unsafe extern "C" fn on_vehicle_repair(
    state: *mut (),
    evt: *mut VehicleRepairEvent,
    host: &'static HostApi,
) -> bool {
    let c = with_cell(state);
    guard_intercept(host, c.name, "on_vehicle_repair", || {
        c.plugin.on_vehicle_repair(&mut *evt, &Ctx::new(host, c.name))
    })
}
unsafe extern "C" fn drop_cell(state: *mut ()) {
    drop(Box::from_raw(state as *mut Cell));
}
