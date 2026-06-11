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
    pub fn log_info(&self, msg: &str) { self.log(LogLevel::Info, msg); }

    /// Log at warn level. Use for suspicious but not-yet-actioned signals.
    ///
    /// 以 warn 级别写日志。适合「可疑但还没动手」的信号。
    pub fn log_warn(&self, msg: &str) { self.log(LogLevel::Warn, msg); }

    /// Log at error level. Use for hard violations or plugin-internal errors.
    ///
    /// 以 error 级别写日志。适合明确违规或插件内部错误。
    pub fn log_error(&self, msg: &str) { self.log(LogLevel::Error, msg); }

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
        self.kv_set_global(&self.namespaced(key), value);
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
    }

    /// Read a value from the shared, un-namespaced KV space. Returns
    /// `None` if the key doesn't exist.
    ///
    /// 从全局共享(不加命名空间)的 KV 空间读取一个值。key 不存在时
    /// 返回 `None`。
    pub fn kv_get_global(&self, key: &str) -> Option<String> {
        let k = CString::new(key).unwrap_or_default();
        let s = read_string_via(|buf, cap| unsafe { (self.host.kv_get)(k.as_ptr(), buf, cap) });
        if s.is_empty() { None } else { Some(s) }
    }

    #[inline]
    fn namespaced(&self, key: &str) -> String {
        format!("{}:{}", self.plugin_name, key)
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
}
