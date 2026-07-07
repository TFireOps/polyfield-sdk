//! The per-call handle plugins use to interact with the framework.
//!
//! 插件在事件回调中收到的运行时句柄。
//!
//! A `Ctx` is effectively a borrowed view of the [`crate::HostApi`] vtable
//! with ergonomic Rust wrappers around the raw C function pointers.
//! Construction is handled by the framework — plugin authors never build
//! one themselves.
//!
//! `Ctx` 本质上是 [`crate::HostApi`] vtable 的一层薄封装，把底层的 C
//! 函数指针包装成更好用的 Rust 方法。实例由框架构造，插件作者不应
//! 手动创建。

use crate::abi::{HostApi, LogLevel};
use crate::events::PlayerRef;
use crate::game_enums::MatchType;
use crate::player::{read_string_via, Player};
use crate::vehicle::Vehicle;
use std::ffi::CString;
use std::path::PathBuf;

/// Read-only snapshot of a single player's state.
///
/// 单个玩家状态的只读快照。
///
/// Produced by the host and returned in bulk from [`Ctx::players`].
/// Fields represent the player's state at the moment the snapshot was
/// taken and do not update in place.
///
/// 由宿主生成，通过 [`Ctx::players`] 成批返回。字段反映的是拍摄快照
/// 时的瞬时状态，不会就地更新。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlayerSnapshot {
    pub id: PlayerRef,
    /// World position `[x, y, z]`.
    ///
    /// 世界坐标 `[x, y, z]`。
    pub pos: [f32; 3],
    pub velocity: [f32; 3],
    pub health: f32,
    /// Most recent latency sample in milliseconds.
    ///
    /// 最近一次延迟采样，单位毫秒。
    pub latency_ms: f32,
}

/// Per-callback context handed to the plugin by the framework.
///
/// 框架在每次回调时传给插件的上下文句柄。
///
/// Not `Send`/`Sync` on purpose — the reference is only valid for the
/// duration of the callback it was passed to. Do not store it, do not
/// move it to another thread, do not spawn a thread that captures it.
/// If you need to act later, record the data you need and act from the
/// next callback or from your own state.
///
/// 刻意没实现 `Send`/`Sync` —— 该引用只在当前回调调用期间有效。
/// 不要把它存起来、不要移到别的线程、也不要起一个捕获它的线程。
/// 如果需要延后处理，请把所需数据记下，下次回调或通过插件自身状态
/// 来触发动作。
pub struct Ctx {
    host: &'static HostApi,
    plugin_name: &'static str,
}

impl Ctx {
    #[doc(hidden)]
    pub fn new(host: &'static HostApi, plugin_name: &'static str) -> Self {
        Self { host, plugin_name }
    }

    /// Log at info level. Prefixed with `[<plugin-name>]` in the unified
    /// `polyfield.log`.
    ///
    /// 以 info 级别写一行日志。统一日志 `polyfield.log` 里会带上
    /// `[<插件名>]` 前缀。
    pub fn log_info(&self, msg: &str) {
        self.log(LogLevel::Info, msg);
    }

    /// Log at warn level. Use for suspicious but not-yet-actioned signals.
    ///
    /// 以 warn 级别写日志。适合「可疑但还没动手」的信号。
    pub fn log_warn(&self, msg: &str) {
        self.log(LogLevel::Warn, msg);
    }

    /// Log at error level. Use for hard violations or plugin-internal errors.
    ///
    /// 以 error 级别写日志。适合明确违规或插件内部错误。
    pub fn log_error(&self, msg: &str) {
        self.log(LogLevel::Error, msg);
    }

    fn log(&self, level: LogLevel, msg: &str) {
        let plugin = CString::new(self.plugin_name).unwrap_or_default();
        let msg = CString::new(msg).unwrap_or_default();
        unsafe { (self.host.log)(level, plugin.as_ptr(), msg.as_ptr()) };
    }

    /// Force the game process to terminate.
    ///
    /// 强制退出游戏进程。
    ///
    /// If a `quit` action is configured in `polyfield.toml`, the
    /// framework calls the game's own quit method (so Unity can flush
    /// whatever it wants to flush). Otherwise it falls back to
    /// `exit(0)`, which gives the game no chance to clean up.
    ///
    /// 若 `polyfield.toml` 配置了 `quit` 动作，框架会调用游戏自身的
    /// 退出方法，让 Unity 有机会做清理。若未配置，则回退到
    /// `exit(0)`，游戏来不及做任何清理就终止。
    pub fn force_quit(&self) {
        unsafe { (self.host.force_quit)() };
    }

    /// Show a modal dialog inside the game.
    ///
    /// 在游戏内弹出一个模态对话框。
    ///
    /// Requires the `show_dialog` action to be configured in
    /// `polyfield.toml`. If not configured, this call logs a warning
    /// and returns immediately.
    ///
    /// 需要在 `polyfield.toml` 中配置 `show_dialog` 动作。未配置时
    /// 该调用只会写一条警告并立即返回。
    pub fn show_dialog(&self, title: &str, body: &str) {
        let title = CString::new(title).unwrap_or_default();
        let body = CString::new(body).unwrap_or_default();
        unsafe { (self.host.show_dialog)(title.as_ptr(), body.as_ptr()) };
    }

    /// Build a [`Player`] handle for the given id.
    ///
    /// 为指定 id 构造一个 [`Player`] 句柄。
    ///
    /// The returned handle is cheap to construct (just a pointer
    /// bundle) and gives ergonomic access to the player's fields and
    /// per-player actions. The handle is bound to this `Ctx`'s
    /// lifetime — it cannot outlive the current callback.
    ///
    /// 返回的句柄构造代价极低（只是封装两个指针），可以方便地访问
    /// 玩家字段和针对该玩家的动作。句柄绑定到当前 `Ctx` 的生命周期，
    /// 不能在事件回调结束后继续使用。
    pub fn player(&self, id: PlayerRef) -> Player<'_> {
        Player::new(id, self.host)
    }

    /// The host's [`Player`] handle (server's local player).
    ///
    /// host（服务器主机）的 [`Player`] 句柄。
    ///
    /// Returns `None` when the host's player isn't available yet —
    /// typically during early lobby/loading state, or if the framework
    /// initialised before the host's `PlayerControl` instance was
    /// constructed.
    ///
    /// host 玩家尚不可用时返回 `None`——通常发生在大厅/加载早期阶段，
    /// 或框架在 host 的 `PlayerControl` 实例创建前就完成初始化。
    ///
    /// Backed by `PlayerControl.get_Local()`.
    pub fn host_player(&self) -> Option<Player<'_>> {
        let id = unsafe { (self.host.host_ref)() };
        (id != 0).then(|| Player::new(id, self.host))
    }

    /// Look up a player by their internal name (`PlayersManager.GetPlayer`).
    /// Returns `None` if no player with that exact name exists.
    ///
    /// 通过内部名查找玩家（`PlayersManager.GetPlayer`）。该名字不存在
    /// 时返回 `None`。
    pub fn player_by_name(&self, name: &str) -> Option<Player<'_>> {
        let c_name = CString::new(name).unwrap_or_default();
        let id = unsafe { (self.host.find_player)(c_name.as_ptr()) };
        (id != 0).then(|| Player::new(id, self.host))
    }

    /// Look up a player by their slot id (1-based). Internally formats
    /// `Player{id}` and calls [`player_by_name`](Self::player_by_name).
    ///
    /// 按槽位 id 查找玩家（从 1 开始）。内部拼接 `Player{id}` 后调用
    /// [`player_by_name`](Self::player_by_name)。
    pub fn player_by_id(&self, id: u32) -> Option<Player<'_>> {
        self.player_by_name(&format!("Player{id}"))
    }

    /// Snapshot all currently tracked players.
    ///
    /// 对当前已追踪的玩家做一次快照。
    ///
    /// Returns an empty `Vec` if the host has not populated the roster
    /// yet (e.g. called before anyone has joined). The returned `Vec`
    /// is a copy; the underlying storage can be reallocated at any
    /// time, so don't hold references into it longer than this scope.
    ///
    /// 若宿主尚未填充玩家名单（例如没人加入时调用），会返回空的
    /// `Vec`。返回值是一份拷贝；底层存储随时可能被重分配，因此
    /// 不要在当前作用域之外持有其内部引用。
    pub fn players(&self) -> Vec<PlayerSnapshot> {
        let mut out: *const PlayerSnapshot = std::ptr::null();
        let mut len: usize = 0;
        unsafe { (self.host.players)(&mut out, &mut len) };
        if out.is_null() || len == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(out, len) }.to_vec()
    }

    // ── Game-level state queries ────────────────────────────────────────────

    /// Current map name, with the suffix after `"-"` stripped (so
    /// `"desert-classic"` → `"desert"`).
    ///
    /// 当前地图名，去掉 `"-"` 之后的后缀（例如 `"desert-classic"`
    /// → `"desert"`）。
    ///
    /// Backed by `GameManager.Instance.GetMapName()`. Returns an empty
    /// string if the singleton or method couldn't be resolved (typically
    /// only during early startup before `GameManager` exists).
    ///
    /// 底层调用 `GameManager.Instance.GetMapName()`。单例或方法未解析
    /// 时（通常只在 `GameManager` 实例化前的极早期启动阶段）返回空串。
    pub fn game_map(&self) -> String {
        read_string_via(|buf, cap| unsafe { (self.host.game_map)(buf, cap) })
    }

    /// Current match type as a string: `"teamMatch"`, `"conquest"`, or
    /// `"unknown:N"` for unrecognised values. Pair with
    /// [`crate::game_enums::MatchType::from_raw`] if you want a typed
    /// pattern-match.
    ///
    /// 当前 match type 字符串：`"teamMatch"` / `"conquest"`，或对未识别值
    /// 返回 `"unknown:N"`。需要类型化匹配时可配合
    /// [`crate::game_enums::MatchType::from_raw`]。
    ///
    /// Backed by reading `GameManager.Instance.matchType`. Empty string
    /// if not yet resolvable.
    ///
    /// 底层读取 `GameManager.Instance.matchType`。尚不可解析时返回空串。
    pub fn match_type(&self) -> String {
        read_string_via(|buf, cap| unsafe { (self.host.match_type)(buf, cap) })
    }

    /// Current match type as a typed [`MatchType`]. `None` when the value
    /// isn't recognised (the `"unknown:N"` case) or isn't resolvable yet.
    ///
    /// 当前 match type 的类型化 [`MatchType`]。值无法识别（`"unknown:N"`）
    /// 或尚不可解析时返回 `None`。
    pub fn match_type_enum(&self) -> Option<MatchType> {
        MatchType::from_name(&self.match_type())
    }

    /// `ServerEntityInspector.Instance.GetAnalytics()` — game-supplied
    /// analytics dump (large string, JSON-ish). Returns an empty string
    /// if the singleton isn't available.
    ///
    /// `ServerEntityInspector.Instance.GetAnalytics()` —— 游戏提供的
    /// analytics dump（较大的 JSON-ish 字符串）。单例不可用时返回空串。
    ///
    /// The buffer can be sizeable; the SDK transparently switches from a
    /// 256-byte stack buffer to a heap allocation if needed.
    ///
    /// 返回字符串可能较大；SDK 内部会在 256 字节栈缓冲不够时透明切换
    /// 到堆分配。
    pub fn entities_inspect(&self) -> String {
        read_string_via(|buf, cap| unsafe { (self.host.entities_inspect)(buf, cap) })
    }

    /// Send a chat message as the host. Calls
    /// `PlayerControl.CmdSendChat` on the host's player; the message is
    /// prefixed with `"(raw)"` so the game broadcasts it without further
    /// processing. Use [`color`](crate::color) to colourise the body.
    ///
    /// 以 host 身份发聊天消息。底层在 host 玩家上调用
    /// `PlayerControl.CmdSendChat`，消息会自动加 `"(raw)"` 前缀以便
    /// 游戏直接广播。需要染色时配合 [`color`](crate::color)。
    pub fn host_say(&self, msg: &str) {
        let c = CString::new(msg).unwrap_or_default();
        unsafe { (self.host.host_say)(c.as_ptr()) };
    }

    // ── Shared KV store ────────────────────────────────────────────────────

    /// Store a value in the framework's shared KV store.
    ///
    /// 向框架的共享 KV 存储写入一个值。
    ///
    /// Keys are **automatically namespaced** with this plugin's name, so
    /// `kv_set("count", ...)` from plugin `foo` writes `foo:count`. Other
    /// plugins cannot read or clobber it via the same short key. Use
    /// [`kv_set_global`](Self::kv_set_global) for a deliberately shared key.
    ///
    /// key 会**自动加上本插件名作为命名空间**：插件 `foo` 调用
    /// `kv_set("count", ...)` 实际写入 `foo:count`，其它插件用同样的短
    /// key 既读不到也覆盖不了。需要刻意共享时用
    /// [`kv_set_global`](Self::kv_set_global)。
    pub fn kv_set(&self, key: &str, value: &str) {
        let k = CString::new(self.namespaced(key)).unwrap_or_default();
        let v = CString::new(value).unwrap_or_default();
        unsafe { (self.host.kv_set)(k.as_ptr(), v.as_ptr()) };
    }

    /// Read a value previously written by **this plugin** via
    /// [`kv_set`](Self::kv_set). Returns `None` if the key doesn't exist.
    ///
    /// 读取**本插件**通过 [`kv_set`](Self::kv_set) 写入的值。key 不存在
    /// 时返回 `None`。
    pub fn kv_get(&self, key: &str) -> Option<String> {
        self.kv_get_global(&self.namespaced(key))
    }

    /// Store a value under a **shared, un-namespaced** key visible to all
    /// plugins. Use this only for deliberate cross-plugin coordination;
    /// prefer [`kv_set`](Self::kv_set) for plugin-private state.
    ///
    /// 写入一个**全局共享、不加命名空间**的 key，所有插件都能看到。
    /// 仅用于有意的跨插件协作；插件私有状态请优先用
    /// [`kv_set`](Self::kv_set)。
    pub fn kv_set_global(&self, key: &str, value: &str) {
        let k = CString::new(key).unwrap_or_default();
        let v = CString::new(value).unwrap_or_default();
        unsafe { (self.host.kv_set)(k.as_ptr(), v.as_ptr()) };

        // Sidecar owner metadata lets the panel show global keys under the
        // plugin that last wrote them without changing the HostApi ABI.
        let owner_key = CString::new(format!("__pf_global_owner:{key}")).unwrap_or_default();
        let owner = CString::new(self.plugin_name).unwrap_or_default();
        unsafe { (self.host.kv_set)(owner_key.as_ptr(), owner.as_ptr()) };
    }

    /// Read a value from the shared, un-namespaced KV space. Returns
    /// `None` if the key doesn't exist.
    ///
    /// 从全局共享(不加命名空间)的 KV 空间读取一个值。key 不存在时
    /// 返回 `None`。
    pub fn kv_get_global(&self, key: &str) -> Option<String> {
        let k = CString::new(key).unwrap_or_default();
        let s = read_string_via(|buf, cap| unsafe { (self.host.kv_get)(k.as_ptr(), buf, cap) });
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    #[inline]
    fn namespaced(&self, key: &str) -> String {
        format!("{}:{}", self.plugin_name, key)
    }

    /// Delete a key previously written by **this plugin** via
    /// [`kv_set`](Self::kv_set).
    ///
    /// 删除**本插件**经 [`kv_set`](Self::kv_set) 写入的 key。
    pub fn kv_del(&self, key: &str) {
        self.kv_del_global(&self.namespaced(key));
    }

    /// Delete a **shared, un-namespaced** key (counterpart of
    /// [`kv_set_global`](Self::kv_set_global)).
    ///
    /// 删除一个**全局共享、不加命名空间**的 key（[`kv_set_global`](Self::kv_set_global)
    /// 的对侧）。
    pub fn kv_del_global(&self, key: &str) {
        let k = CString::new(key).unwrap_or_default();
        unsafe { (self.host.kv_del)(k.as_ptr()) };

        let owner_key = CString::new(format!("__pf_global_owner:{key}")).unwrap_or_default();
        unsafe { (self.host.kv_del)(owner_key.as_ptr()) };
    }

    /// Delete every shared key whose name starts with `prefix` — bulk reset of
    /// a key family. `prefix` is matched verbatim (no plugin namespace added),
    /// so a plugin can clear a global family like `"pf:kicked_dev:"`.
    ///
    /// 删除所有以 `prefix` 开头的全局 key——批量重置一族。`prefix` 按原样匹配
    /// （不加插件命名空间），因此可清理 `"pf:kicked_dev:"` 这类全局族。
    pub fn kv_clear_prefix(&self, prefix: &str) {
        let p = CString::new(prefix).unwrap_or_default();
        unsafe { (self.host.kv_clear_prefix)(p.as_ptr()) };
    }

    // ── Vehicle field readers ──────────────────────────────────────────────

    /// Read `VehicleControl.health`.
    ///
    /// Prefer the [`Vehicle`] handle: `ctx.vehicle(id).health()`.
    #[deprecated(note = "use ctx.vehicle(id).health()")]
    pub fn vehicle_health(&self, vehicle: PlayerRef) -> i32 {
        unsafe { (self.host.vehicle_health)(vehicle) }
    }

    /// Read `VehicleControl.vehicleType` as raw i32.
    ///
    /// Prefer the [`Vehicle`] handle: `ctx.vehicle(id).vehicle_type()`.
    #[deprecated(note = "use ctx.vehicle(id).vehicle_type()")]
    pub fn vehicle_type(&self, vehicle: PlayerRef) -> i32 {
        unsafe { (self.host.vehicle_type)(vehicle) }
    }

    /// Read `VehicleControl.myDriver` → PlayerRef. Returns 0 if no driver.
    ///
    /// Prefer the [`Vehicle`] handle: `ctx.vehicle(id).driver()`.
    #[deprecated(note = "use ctx.vehicle(id).driver()")]
    pub fn vehicle_driver(&self, vehicle: PlayerRef) -> PlayerRef {
        unsafe { (self.host.vehicle_driver)(vehicle) }
    }

    /// Get all currently active vehicles as `Vehicle` handles.
    ///
    /// 获取当前所有活跃载具的 `Vehicle` 句柄。
    pub fn vehicles(&self) -> Vec<Vehicle<'_>> {
        let mut out: *const PlayerRef = std::ptr::null();
        let mut len: usize = 0;
        unsafe { (self.host.vehicles)(&mut out, &mut len) };
        if out.is_null() || len == 0 {
            return Vec::new();
        }
        let refs = unsafe { std::slice::from_raw_parts(out, len) };
        refs.iter().map(|&id| Vehicle::new(id, self.host)).collect()
    }

    /// Build a `Vehicle` handle from a raw VehicleControl pointer.
    ///
    /// 从原始 VehicleControl 指针构造 `Vehicle` 句柄。
    pub fn vehicle(&self, id: PlayerRef) -> Vehicle<'_> {
        Vehicle::new(id, self.host)
    }

    // ── Roster (live handles) ───────────────────────────────────────────────

    /// Every online player as a live [`Player`] handle.
    ///
    /// 所有在线玩家的实时 [`Player`] 句柄。
    ///
    /// Unlike [`players`](Self::players), which returns immutable
    /// [`PlayerSnapshot`]s with a fixed field set, these handles read the
    /// game's current state on demand and expose the full accessor +
    /// action surface. This is what server-wide scans want — speed
    /// checks, load-gating, per-team messaging, kd tallies.
    ///
    /// 与返回固定字段只读 [`PlayerSnapshot`] 的 [`players`](Self::players)
    /// 不同，这里的句柄按需读取游戏当前状态，并暴露完整的 getter +
    /// 动作能力。全场扫描（加速检测、加载限流、按队伍发消息、kd 统计）
    /// 应该用它。
    ///
    /// Returns an empty `Vec` if the roster isn't populated yet. Handles
    /// are bound to this `Ctx` and must not outlive the callback.
    ///
    /// 名单尚未填充时返回空 `Vec`。句柄绑定到当前 `Ctx`，不能在回调
    /// 结束后继续使用。
    pub fn all_players(&self) -> Vec<Player<'_>> {
        let mut out: *const PlayerRef = std::ptr::null();
        let mut len: usize = 0;
        unsafe { (self.host.all_players)(&mut out, &mut len) };
        if out.is_null() || len == 0 {
            return Vec::new();
        }
        let refs = unsafe { std::slice::from_raw_parts(out, len) };
        refs.iter().map(|&id| Player::new(id, self.host)).collect()
    }

    // ── Match state read/write ──────────────────────────────────────────────

    /// `GameManager.Instance.currentTime` — the match countdown in
    /// seconds. `None` if `GameManager` isn't available yet.
    ///
    /// `GameManager.Instance.currentTime` —— 对局倒计时（秒）。
    /// `GameManager` 尚不可用时返回 `None`。
    pub fn current_time(&self) -> Option<f32> {
        let t = unsafe { (self.host.game_current_time)() };
        (t >= 0.0).then_some(t)
    }

    /// Force the match countdown to `secs`. Setting a small value (e.g.
    /// `10.0`) is how vote-map / admin-rotate features trigger an early
    /// map change. No-op if `GameManager` isn't available.
    ///
    /// 强制把对局倒计时设为 `secs`。设一个小值（如 `10.0`）即是投票
    /// 换图 / 管理员强制换图触发提前换图的方式。`GameManager` 不可用
    /// 时为空操作。
    pub fn set_current_time(&self, secs: f32) {
        unsafe { (self.host.game_set_current_time)(secs) };
    }

    /// `GameManager.Instance.DAMAGE_FACTOR` — the server-wide integer damage
    /// multiplier. Actual HP removed by a hit is `rpc_damage * damage_factor`
    /// (see `ClientKillLogics`). Returns `1` when unavailable, so multiplying
    /// by it is always safe.
    ///
    /// `GameManager.Instance.DAMAGE_FACTOR` —— 服务器全局整数伤害乘数。
    /// 一次命中实际扣血 = `rpc 伤害 * damage_factor`（见 `ClientKillLogics`）。
    /// 不可用时返回 `1`，乘它始终安全。
    pub fn damage_factor(&self) -> i32 {
        unsafe { (self.host.game_damage_factor)() }
    }

    /// `GameUtility.GetExplosionDamage(pos, dmgType, weaponID)` — 服务端按真实
    /// 实体位置**重算一次**爆炸的多目标伤害,返回游戏原样的
    /// `"\n名字:伤害\n..."` 字符串。炮弹/手雷/火箭筒的范围伤害由游戏编码在
    /// 伤害 RPC 的 `_data` 里(而非 `_dmg`);插件可在 `on_damage` 里用本方法
    /// 重算、按倍率缩放后写回 `evt.data`。`dmg_type` 为 [`crate::game_enums::DamageType`]
    /// 原始值,`weapon_id` 对火箭筒为 GadgetModel。类/方法未解析时返回空串。
    ///
    /// **仅在游戏线程调用**(`on_damage` 由 RPC hook 在游戏线程触发,安全);
    /// 它内部遍历实体并做物理射线,不可从其它线程调用。
    pub fn explosion_damage(&self, pos: [f32; 3], dmg_type: i32, weapon_id: i32) -> String {
        read_string_via(|buf, cap| unsafe {
            (self.host.game_explosion_damage)(pos.as_ptr(), dmg_type, weapon_id, buf, cap)
        })
    }

    // ── Map control ──────────────────────────────────────────────────────────

    /// The server's configured **map pool** — the `match map` list from
    /// `ServerConfig.txt` (comma-separated in the file, split for you).
    /// Read-only; the framework never rewrites the operator's config. Empty
    /// `Vec` if no pool is configured. This is the set of names accepted by
    /// [`set_next_map`](Self::set_next_map).
    ///
    /// 服务器配置的**地图池** —— `ServerConfig.txt` 的 `match map` 列表
    /// （文件里逗号分隔，已替你切分）。只读；框架绝不改写运营者配置。
    /// 未配置地图池时返回空 `Vec`。这也是 [`set_next_map`](Self::set_next_map)
    /// 接受的名字集合。
    ///
    /// Cheap file read; fine from `on_command` / occasional checks — don't
    /// call it every tick.
    ///
    /// 轻量文件读取；适合 `on_command` / 偶尔检查——别每帧调。
    pub fn server_maps(&self) -> Vec<String> {
        read_string_via(|buf, cap| unsafe { (self.host.server_maps)(buf, cap) })
            .split('\n')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// Queue the map for the **next** round. `name` must be one of
    /// [`server_maps`](Self::server_maps) — returns `false` (no change) if it
    /// isn't. An empty `name` clears any pending override and returns `true`.
    ///
    /// **Set-only**: the change applies when the match next rotates (the
    /// framework intercepts the game's per-round map assignment). To switch
    /// immediately, pair with
    /// [`set_current_time(0.0)`](Self::set_current_time) to force the current
    /// match to end now. Never modifies `ServerConfig.txt` or `PlayerPrefs`,
    /// so the operator's pool stays intact. The override is **one-shot** —
    /// after it applies to one round, the configured pool resumes.
    ///
    /// 把地图排队为**下一局**。`name` 必须是 [`server_maps`](Self::server_maps)
    /// 之一，否则返回 `false`（不改动）。`name` 为空则清除待生效覆盖并返回
    /// `true`。
    ///
    /// **仅设置**：下次对局轮换时生效（框架拦截游戏每局的地图赋值）。要立即
    /// 切换，请配合 [`set_current_time(0.0)`](Self::set_current_time) 强制结束
    /// 当前对局。绝不修改 `ServerConfig.txt` 或 `PlayerPrefs`，运营者地图池
    /// 保持原样。覆盖是**一次性**的——应用到一局后，配置的地图池随后恢复。
    pub fn set_next_map(&self, name: &str) -> bool {
        let c = CString::new(name).unwrap_or_default();
        unsafe { (self.host.server_set_next_map)(c.as_ptr()) }
    }

    /// The map currently queued by [`set_next_map`](Self::set_next_map) for
    /// the next round, or `None` if nothing is queued.
    ///
    /// 当前经 [`set_next_map`](Self::set_next_map) 排队、将在下一局生效的
    /// 地图，无则 `None`。
    pub fn next_map(&self) -> Option<String> {
        let s = read_string_via(|buf, cap| unsafe { (self.host.server_next_map)(buf, cap) });
        (!s.is_empty()).then_some(s)
    }

    // ── Server-list info (v29) ───────────────────────────────────────────────
    //
    // The data the game's own `ShareServer` announce uses. Exposed so a plugin
    // can take over list-keepalive without the operator re-configuring values
    // that already live in the game (serverLink/region/version/port/max). All
    // degrade to empty / `None` when unresolved, so a plugin can fall back to
    // its own config.

    /// `GameManager.serverLink` — the public list-server endpoint URL. A Unity
    /// serialized field (not in the DLL), so this is the only way a plugin can
    /// obtain it. Empty string if unresolved.
    ///
    /// `GameManager.serverLink` —— 公共列表服务器的上报 URL。它是 Unity 序列化
    /// 字段（不在 DLL 里），插件只能经此获取。未解析时返回空串。
    pub fn server_link(&self) -> String {
        read_string_via(|buf, cap| unsafe { (self.host.server_link)(buf, cap) })
    }

    /// Server region — `PlayerPrefs["Region"]`, or `"Unknown"`.
    ///
    /// 服务器区服 —— `PlayerPrefs["Region"]`，缺省 `"Unknown"`。
    pub fn server_region(&self) -> String {
        read_string_via(|buf, cap| unsafe { (self.host.server_region)(buf, cap) })
    }

    /// `Application.version` — the game build version string. Empty if
    /// unresolved.
    ///
    /// `Application.version` —— 游戏构建版本号。未解析时返回空串。
    pub fn game_version(&self) -> String {
        read_string_via(|buf, cap| unsafe { (self.host.game_version)(buf, cap) })
    }

    /// Transport listen port (`KcpTransport.port`). `None` if unresolved.
    ///
    /// 传输层监听端口（`KcpTransport.port`）。未解析时返回 `None`。
    pub fn server_port(&self) -> Option<u16> {
        let p = unsafe { (self.host.server_port)() };
        (p != 0).then_some(p)
    }

    /// `NetworkManager.singleton.maxConnections` — the server's max player
    /// slots. `None` if unresolved.
    ///
    /// `NetworkManager.singleton.maxConnections` —— 服务器最大人数槽位。
    /// 未解析时返回 `None`。
    pub fn max_players(&self) -> Option<u32> {
        let n = unsafe { (self.host.max_players)() };
        (n != 0).then_some(n)
    }

    /// `NetworkManager.singleton.numPlayers` — current authenticated player
    /// count. `None` if unresolved (fall back to `all_players().len()`); a real
    /// count of `0` is returned as `Some(0)`.
    ///
    /// `NetworkManager.singleton.numPlayers` —— 当前已认证玩家数。未解析时返回
    /// `None`（可回退 `all_players().len()`）；真实人数为 0 时返回 `Some(0)`。
    pub fn player_count(&self) -> Option<u32> {
        let n = unsafe { (self.host.player_count)() };
        (n != u32::MAX).then_some(n)
    }

    // ── Backend channel ─────────────────────────────────────────────────────

    /// Emit a structured event to the management backend through the
    /// host. `kind` is a short type tag (e.g. `"kickCheat"`, `"ban"`);
    /// `json` is an opaque UTF-8 payload — typically serialized JSON the
    /// host forwards verbatim. The inbound counterpart is
    /// [`Plugin::on_command`](crate::Plugin::on_command).
    ///
    /// 经宿主向管理后端发送一条结构化事件。`kind` 是简短类型标签
    /// （如 `"kickCheat"`、`"ban"`）；`json` 是宿主原样转发的 UTF-8
    /// 透传负载（通常是序列化后的 JSON）。入站对应物是
    /// [`Plugin::on_command`](crate::Plugin::on_command)。
    pub fn emit(&self, kind: &str, json: &str) {
        let k = CString::new(kind).unwrap_or_default();
        let j = CString::new(json).unwrap_or_default();
        unsafe { (self.host.emit)(k.as_ptr(), j.as_ptr()) };
    }

    // ── Deferred scheduling ─────────────────────────────────────────────────

    /// Arm a one-shot timer. After `delay_ms`, the framework calls this
    /// plugin's [`Plugin::on_timer`](crate::Plugin::on_timer) with
    /// `token`. Use distinct tokens to tell timers apart.
    ///
    /// 注册一次性定时器。`delay_ms` 之后框架用 `token` 调用本插件的
    /// [`Plugin::on_timer`](crate::Plugin::on_timer)。用不同 token 区分
    /// 多个定时器。
    ///
    /// For *periodic* work, prefer dividing down [`Plugin::on_tick`](crate::Plugin::on_tick) by a
    /// frame counter — this is strictly for deferred one-shots (e.g.
    /// "re-check this player in 3 seconds"). No cancellation in v22.
    ///
    /// *周期性* 工作请优先用帧计数对 [`Plugin::on_tick`](crate::Plugin::on_tick) 分频——这里
    /// 仅用于延后的一次性动作（如「3 秒后复查该玩家」）。v22 不支持取消。
    pub fn schedule_once(&self, delay_ms: u64, token: u64) {
        unsafe { (self.host.schedule_once)(delay_ms, token) };
    }

    // ── Per-plugin config & storage ─────────────────────────────────────────
    //
    // Resolved relative to the game's working directory — the same base
    // `polyfield.toml` / `plugins/` use. The framework never reads or writes
    // these; it only hands the plugin a namespaced path. Format and contents
    // are entirely the plugin's own.
    //
    // 相对游戏工作目录解析——与 `polyfield.toml` / `plugins/` 同一基准。框架
    // 既不读也不写这些内容，只给插件一个按插件名隔离的路径。格式与内容完全
    // 由插件自定。

    /// Path to this plugin's config file, `config/<plugin-name>.toml`.
    /// Read-only by convention (authored by the server operator). Load it
    /// with [`read_config`](Self::read_config) and parse with your own
    /// `toml`/`serde` types. For a non-TOML layout, ignore this and build a
    /// name under [`config_dir`](Self::config_dir).
    ///
    /// 本插件的配置文件路径 `config/<插件名>.toml`。按约定只读（由服务器
    /// 运营者编写）。用 [`read_config`](Self::read_config) 读取后，用你自己的
    /// `toml`/`serde` 类型解析。需要非 TOML 布局时忽略它，在
    /// [`config_dir`](Self::config_dir) 下自拼文件名。
    pub fn config_path(&self) -> PathBuf {
        PathBuf::from("config").join(format!("{}.toml", self.plugin_name))
    }

    /// The shared `config/` directory. Escape hatch for a non-default file
    /// name or format — e.g. `ctx.config_dir().join("rules.json")`.
    ///
    /// 共享的 `config/` 目录。给想用非默认文件名/格式的插件的逃生舱，
    /// 例如 `ctx.config_dir().join("rules.json")`。
    pub fn config_dir(&self) -> PathBuf {
        PathBuf::from("config")
    }

    /// Read this plugin's config file ([`config_path`](Self::config_path)) as
    /// a string. `None` if it doesn't exist or can't be read; parse the text
    /// yourself. Cheap from `on_load`; don't call it every tick.
    ///
    /// 把本插件的配置文件（[`config_path`](Self::config_path)）读成字符串。
    /// 文件不存在或读取失败时返回 `None`；文本自行解析。适合在 `on_load`
    /// 调用，别每帧调。
    pub fn read_config(&self) -> Option<String> {
        std::fs::read_to_string(self.config_path()).ok()
    }

    /// This plugin's private storage directory, `data/<plugin-name>/`. The
    /// directory is created if missing. Read, write, and create files here
    /// freely — contents and format are the plugin's own (a JSON state file,
    /// a SQLite db, whatever). Each plugin gets its own directory; isolation
    /// is by convention (plugins are trusted code).
    ///
    /// **Threading:** the returned `PathBuf` is `Send` — store it and do
    /// heavy/frequent writes from your own background thread to avoid
    /// blocking the game's main thread inside event callbacks.
    ///
    /// 本插件的私有存储目录 `data/<插件名>/`。目录不存在会被创建。在这里
    /// 自由读写创建文件——内容和格式由插件自定（JSON 状态文件、SQLite 库
    /// 等）。每个插件有各自的目录；隔离按约定（插件是可信代码）。
    ///
    /// **线程：** 返回的 `PathBuf` 是 `Send`——可存下它、在自己的后台线程做
    /// 重/频繁写入，避免在事件回调里阻塞游戏主线程。
    pub fn data_dir(&self) -> PathBuf {
        let dir = PathBuf::from("data").join(self.plugin_name);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }
}
