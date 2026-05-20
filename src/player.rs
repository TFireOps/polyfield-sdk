//! Read-only and action-capable handle to a single `PlayerControl` instance.
//!
//! 单个 `PlayerControl` 实例的只读 + 动作句柄。
//!
//! `Player<'_>` is the high-level surface plugin authors should use.
//! Internally it wraps an opaque `PlayerRef` (currently the
//! `PlayerControl` instance pointer) and a borrowed [`HostApi`] vtable;
//! every accessor delegates to a generic field reader, while typed
//! Rust values come back to the caller.
//!
//! `Player<'_>` 是插件作者主要使用的高层句柄。内部封装了不透明的
//! `PlayerRef`（当前是 `PlayerControl` 实例指针）和借用的 [`HostApi`]
//! vtable；每个 getter 在底层都通过通用字段读取器读字段，但回到
//! Rust 这边时已经是带类型的值。
//!
//! # Stability / 稳定性
//!
//! Adding a new accessor here does **not** require bumping
//! `POLYFIELD_ABI_VERSION` — it only adds a method to the SDK and
//! a new offset entry in the host. Plugins built against an older
//! SDK will simply be missing the newer accessors.
//!
//! 在这里新增 getter **不**需要 bump `POLYFIELD_ABI_VERSION`——只是
//! 给 SDK 加一个方法、给宿主加一行偏移登记。基于旧 SDK 编译的插件
//! 仅会缺少新方法。
//!
//! # Lifetime / 生命周期
//!
//! Cannot outlive the `Ctx` it was created from. This matches the
//! callback contract: a `Player` is valid for the duration of the
//! event handler that produced it.
//!
//! 生命周期不能超过创建它的 `Ctx`。这和回调契约一致：一个 `Player`
//! 句柄只在产生它的事件处理函数内有效。
//!
//! [`HostApi`]: crate::HostApi

use std::ffi::CString;
use std::os::raw::c_char;

use crate::abi::HostApi;
use crate::events::PlayerRef;
use crate::fields as f;

/// Handle for interacting with a single player.
///
/// 用于操作单个玩家的句柄。
///
/// Construct via [`crate::Ctx::player`] or one of the per-event
/// helpers like [`crate::events::PlayerJoinEvent::player`]. Methods
/// that read game state may return defaults (`0`, `0.0`, empty
/// string, `false`, `[0,0,0]`) if the underlying `PlayerControl`
/// no longer exists or the field couldn't be resolved on this game build.
///
/// 通过 [`crate::Ctx::player`] 或事件提供的 helper（例如
/// [`crate::events::PlayerJoinEvent::player`]）构造。读取游戏状态的
/// 方法在底层 `PlayerControl` 已不存在、或当前游戏构建中字段无法解析
/// 时，会返回默认值（`0` / `0.0` / 空串 / `false` / `[0,0,0]`）。
#[derive(Clone, Copy)]
pub struct Player<'ctx> {
    pub(crate) id: PlayerRef,
    pub(crate) host: &'ctx HostApi,
}

impl<'ctx> Player<'ctx> {
    #[doc(hidden)]
    pub fn new(id: PlayerRef, host: &'ctx HostApi) -> Self {
        Self { id, host }
    }

    /// Opaque identifier, stable for the duration of the current
    /// session. Currently the `PlayerControl` instance pointer.
    ///
    /// 不透明标识，本局会话内保持稳定。当前实现是 `PlayerControl` 实例
    /// 指针。
    pub fn id(&self) -> PlayerRef {
        self.id
    }

    // ── String fields ────────────────────────────────────────────────────

    /// The player's **slot id** — the integer N from the GameObject
    /// name `"Player<N>"`. Returns `0` if the name doesn't match the
    /// expected `Player<N>` shape (e.g. host's player object on
    /// some Unity builds, or a non-player object).
    ///
    /// 玩家**槽位 id**——GameObject name `"Player<N>"` 中的整数 N。
    /// 名字不匹配 `Player<N>` 形式时返回 `0`（例如某些 Unity 构建里
    /// host 自己的玩家对象，或非玩家对象）。
    pub fn player_id(&self) -> u32 {
        self.unity_name()
            .strip_prefix("Player")
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(0)
    }

    /// The player's **editable display name** (from the `_playerID`
    /// field). Defaults to `"Player<N>"` if the player hasn't changed
    /// it. Empty string if the field is null/unresolved.
    ///
    /// 玩家**可编辑的显示名**（来自 `_playerID` 字段）。玩家未改名
    /// 时默认是 `"Player<N>"`。字段为 null 或解析失败时为空串。
    pub fn name(&self) -> String { self.read_string(f::F_PLAYER_ID) }

    /// `deviceID` — fingerprint identifier reported by the client.
    ///
    /// `deviceID` —— 客户端上报的设备指纹标识。
    pub fn device_id(&self) -> String { self.read_string(f::F_DEVICE_ID) }

    /// `team` — team tag string.
    ///
    /// `team` —— 队伍标签字符串。
    pub fn team(&self) -> String { self.read_string(f::F_TEAM) }

    /// `groundType` — surface tag of whatever the player is standing on.
    ///
    /// `groundType` —— 玩家脚下表面的标签。
    pub fn ground_type(&self) -> String { self.read_string(f::F_GROUND_TYPE) }

    // ── Lifecycle / state ───────────────────────────────────────────────

    /// `health` — current HP.
    pub fn health(&self) -> i32 { self.read_i32(f::F_HEALTH) }
    /// `healthRegenCooldown` — seconds remaining before HP regen.
    pub fn health_regen_cooldown(&self) -> f32 { self.read_f32(f::F_HEALTH_REGEN_COOLDOWN) }
    /// `dead` flag — `true` if the game flagged the player as dead,
    /// or if their health has dropped to `0` or below. The latter
    /// catches the brief window between damage application and the
    /// game flipping the `dead` flag.
    ///
    /// `dead` 标志——游戏侧标记死亡时返回 `true`，或血量降到 `0`
    /// 及以下时也返回 `true`。后者捕捉了「伤害已生效但 `dead` 标志
    /// 还没翻」的短暂时间窗口。
    pub fn is_dead(&self) -> bool {
        self.read_bool(f::F_DEAD) || self.health() <= 0
    }
    /// `ready` — ready-up state in lobby/match flow.
    pub fn is_ready(&self) -> bool { self.read_bool(f::F_READY) }
    /// `respawnTimer` — seconds until respawn.
    pub fn respawn_timer(&self) -> f32 { self.read_f32(f::F_RESPAWN_TIMER) }
    /// `doneLoadingMap` — finished loading the current map.
    pub fn done_loading_map(&self) -> bool { self.read_bool(f::F_DONE_LOADING_MAP) }
    /// `myState` — UserState enum (raw integer).
    pub fn user_state(&self) -> i32 { self.read_i32(f::F_USER_STATE) }
    /// `myClass` — ClassRole enum (raw integer).
    pub fn class_role(&self) -> i32 { self.read_i32(f::F_CLASS_ROLE) }

    // ── Stats / counters ────────────────────────────────────────────────

    pub fn kill_count(&self) -> i32 { self.read_i32(f::F_KILL_COUNT) }
    pub fn death_count(&self) -> i32 { self.read_i32(f::F_DEATH_COUNT) }
    pub fn bullets_fired(&self) -> i32 { self.read_i32(f::F_BULLETS_FIRED) }
    pub fn grenades_thrown(&self) -> i32 { self.read_i32(f::F_GRENADES_THROWN) }
    pub fn reloads_done(&self) -> i32 { self.read_i32(f::F_RELOADS_DONE) }
    pub fn kill_rate(&self) -> i32 { self.read_i32(f::F_KILL_RATE) }
    pub fn damage_rate(&self) -> i32 { self.read_i32(f::F_DAMAGE_RATE) }
    pub fn network_rate(&self) -> i32 { self.read_i32(f::F_NETWORK_RATE) }
    pub fn latency_rate(&self) -> i32 { self.read_i32(f::F_LATENCY_RATE) }
    pub fn ping_warn(&self) -> i32 { self.read_i32(f::F_PING_WARN) }
    pub fn teamkill_warn(&self) -> i32 { self.read_i32(f::F_TEAMKILL_WARN) }

    // ── Movement / pose ─────────────────────────────────────────────────

    pub fn is_running(&self) -> bool { self.read_bool(f::F_RUNNING) }
    pub fn is_grounded(&self) -> bool { self.read_bool(f::F_GROUNDED) }
    /// `crouch` — crouch state level (int).
    pub fn crouch(&self) -> i32 { self.read_i32(f::F_CROUCH) }
    pub fn is_under_water(&self) -> bool { self.read_bool(f::F_IS_UNDER_WATER) }

    /// World position from `_netTransform._recivedPos`.
    /// Falls back to `lastPlayerPos` if the net transform is unavailable.
    ///
    /// 取自 `_netTransform._recivedPos` 的世界坐标。
    /// 若 net transform 不可用则回退到 `lastPlayerPos`。
    pub fn position(&self) -> [f32; 3] {
        let v = self.read_vec3(f::F_NET_POSITION);
        if v == [0.0, 0.0, 0.0] {
            return self.read_vec3(f::F_LAST_PLAYER_POS);
        }
        v
    }

    /// Velocity from `_netTransform._recivedVel`.
    /// Falls back to `myRigidVel` if the net transform is unavailable.
    ///
    /// 取自 `_netTransform._recivedVel` 的速度向量。
    /// 若 net transform 不可用则回退到 `myRigidVel`。
    pub fn velocity(&self) -> [f32; 3] {
        let v = self.read_vec3(f::F_NET_VELOCITY);
        if v == [0.0, 0.0, 0.0] {
            return self.read_vec3(f::F_MY_RIGID_VEL);
        }
        v
    }

    /// `_moveDir` — current movement direction vector.
    pub fn move_dir(&self) -> [f32; 3] { self.read_vec3(f::F_MOVE_DIR) }

    /// `_lookDir` — 2D look direction.
    pub fn look_dir(&self) -> [f32; 2] { self.read_vec2(f::F_LOOK_DIR) }

    // ── Combat-adjacent ─────────────────────────────────────────────────

    pub fn trying_to_attack(&self) -> f32 { self.read_f32(f::F_TRYING_TO_ATTACK) }
    pub fn obstacle_timer(&self) -> f32 { self.read_f32(f::F_OBSTACLE_TIMER) }
    pub fn expose_timer(&self) -> f32 { self.read_f32(f::F_EXPOSE_TIMER) }
    pub fn dont_expose(&self) -> bool { self.read_bool(f::F_DONT_EXPOSE) }

    // ── Input / camera ──────────────────────────────────────────────────

    pub fn mouse_x(&self) -> f32 { self.read_f32(f::F_MOUSE_X) }
    pub fn mouse_y(&self) -> f32 { self.read_f32(f::F_MOUSE_Y) }
    pub fn input_x(&self) -> f32 { self.read_f32(f::F_INPUT_X) }
    pub fn input_y(&self) -> f32 { self.read_f32(f::F_INPUT_Y) }
    pub fn auto_sprint(&self) -> bool { self.read_bool(f::F_AUTO_SPRINT) }
    pub fn head_bob(&self) -> bool { self.read_bool(f::F_HEAD_BOB) }
    pub fn joystick_lean(&self) -> bool { self.read_bool(f::F_JOYSTICK_LEAN) }
    pub fn cam_sensitivity(&self) -> f32 { self.read_f32(f::F_CAM_SENSITIVITY) }
    pub fn ads_sensitivity(&self) -> f32 { self.read_f32(f::F_ADS_SENSITIVITY) }
    pub fn gyro_look_sensitivity(&self) -> f32 { self.read_f32(f::F_GYRO_LOOK_SENSITIVITY) }
    pub fn gyro_ads_sensitivity(&self) -> f32 { self.read_f32(f::F_GYRO_ADS_SENSITIVITY) }
    pub fn local_cam_dist(&self) -> f32 { self.read_f32(f::F_LOCAL_CAM_DIST) }
    pub fn cam_fov(&self) -> f32 { self.read_f32(f::F_CAM_FOV) }
    pub fn cam_shake(&self) -> f32 { self.read_f32(f::F_CAM_SHAKE) }
    pub fn default_lod_bias(&self) -> f32 { self.read_f32(f::F_DEFAULT_LOD_BIAS) }

    // ── Network / latency ───────────────────────────────────────────────

    /// `myLatency` — round-trip latency in milliseconds, as the game
    /// most recently sampled it.
    ///
    /// `myLatency` —— 最近一次采样的往返延迟，单位毫秒。
    pub fn latency(&self) -> f32 { self.read_f32(f::F_MY_LATENCY) }

    /// Network IP address from `connectionToClient.address`.
    /// Returns an empty string if the connection isn't established.
    ///
    /// 来自 `connectionToClient.address` 的网络 IP 地址。连接未建立
    /// 时返回空串。
    pub fn ip(&self) -> String {
        read_string_via(|buf, cap| unsafe {
            (self.host.player_ip)(self.id, buf, cap)
        })
    }

    /// Currently equipped weapon id from `playerCombat.currWeaponID`.
    /// Pair with [`crate::game_enums::WeaponId::from_raw`] for a typed view.
    ///
    /// 当前装备武器 ID（来自 `playerCombat.currWeaponID`）。可配合
    /// [`crate::game_enums::WeaponId::from_raw`] 转为带类型的枚举。
    pub fn weapon_id(&self) -> i32 { self.read_i32(f::F_WEAPON_ID) }

    /// GameObject name (slot identifier set at spawn — e.g. `"Player3"`).
    /// **Different from [`name`](Self::name), which is the editable
    /// display name from `_playerID`.** This is what
    /// `PlayersManager.GetPlayer` looks up by, and what
    /// [`player_id`](Self::player_id) parses for the slot integer.
    ///
    /// GameObject 的 name（spawn 时设置的槽位标识，例如 `"Player3"`）。
    /// **与 [`name`](Self::name) 不同，后者读的是 `_playerID` 字段，
    /// 那是玩家可编辑的显示名。** 这才是 `PlayersManager.GetPlayer`
    /// 的查找键，也是 [`player_id`](Self::player_id) 用来解析槽位
    /// 整数的来源。
    ///
    /// Backed by `UnityEngine.Object.get_name()`.
    pub fn unity_name(&self) -> String {
        read_string_via(|buf, cap| unsafe {
            (self.host.player_unity_name)(self.id, buf, cap)
        })
    }

    // ── Voting ──────────────────────────────────────────────────────────

    pub fn vote_kicked(&self) -> bool { self.read_bool(f::F_VOTE_KICKED) }
    pub fn voted(&self) -> bool { self.read_bool(f::F_VOTED) }

    // ── Game-specific actions ───────────────────────────────────────────

    /// `true` if this `PlayerControl` is the local player (the host's
    /// own player object when running on the host machine).
    ///
    /// 如果该 `PlayerControl` 是本地玩家（即在主机上运行时主机自己的
    /// 玩家对象）则返回 `true`。
    ///
    /// Backed by `PlayerControl.get_isLocalPlayer()`.
    ///
    /// 底层调用 `PlayerControl.get_isLocalPlayer()`。
    pub fn is_host(&self) -> bool {
        unsafe { (self.host.player_is_host)(self.id) }
    }

    /// Remotely set this player's health. Calls
    /// `PlayerControl.RpcUpdateHealth(health, flag)`.
    ///
    /// 远程设置该玩家的血量。调用
    /// `PlayerControl.RpcUpdateHealth(health, flag)`。
    pub fn set_health(&self, health: i32, flag: i32) {
        unsafe { (self.host.player_set_health)(self.id, health, flag) };
    }

    /// Show an error panel on this player's client. The panel is the
    /// same modal the game shows when a connection drops, etc.
    ///
    /// 在该玩家的客户端弹出错误面板（与游戏自身在断线等情况下显示的
    /// 模态框相同）。
    ///
    /// Backed by `PlayerControl.RpcErrorPanel(title, body)`.
    ///
    /// 底层调用 `PlayerControl.RpcErrorPanel(title, body)`。
    pub fn show_error(&self, title: &str, body: &str) {
        let t = CString::new(title).unwrap_or_default();
        let b = CString::new(body).unwrap_or_default();
        unsafe { (self.host.player_show_error)(self.id, t.as_ptr(), b.as_ptr()) };
    }

    /// Kill this player. Calls `PlayerControl.RpcKillMe()`.
    ///
    /// 杀死该玩家。调用 `PlayerControl.RpcKillMe()`。
    pub fn kill(&self) {
        unsafe { (self.host.player_kill)(self.id) };
    }

    /// Schedule a kick by calling
    /// `MonoBehaviour.Invoke("KickMePlz", delay_secs)` on this player.
    /// The game's own `KickMePlz` handler does the actual disconnect
    /// after the delay. Use a small delay (e.g. `0.5`) to give a
    /// preceding `show_error` time to render.
    ///
    /// 通过在该玩家上调用
    /// `MonoBehaviour.Invoke("KickMePlz", delay_secs)` 安排踢出。
    /// 游戏自己的 `KickMePlz` handler 会在延迟后执行真正的断开。
    /// 配合前一个 `show_error` 时建议用一点延迟（例如 `0.5`），
    /// 让弹窗有时间渲染出来。
    pub fn kick_me(&self, delay_secs: f32) {
        unsafe { (self.host.player_kick_me)(self.id, delay_secs) };
    }

    /// Convenience: show an error panel **and then** schedule a kick.
    ///
    /// 便捷方法：先弹错误面板**再**安排踢出。
    ///
    /// Equivalent to:
    /// ```ignore
    /// p.show_error(title, body);
    /// p.kick_me(delay_secs);
    /// ```
    ///
    /// The order matters — the error panel needs to be RPC'd to the
    /// client before the disconnect fires, otherwise the client never
    /// sees the reason. A small delay (recommended: `0.5`s) gives the
    /// panel time to render.
    ///
    /// 等价于先调 `show_error` 再调 `kick_me`。顺序很重要——错误面板
    /// 必须先 RPC 给客户端，再触发断开，否则客户端看不到理由。
    /// 推荐用 `0.5` 秒延迟，让面板有时间渲染。
    pub fn kick_with_reason(&self, title: &str, body: &str, delay_secs: f32) {
        self.show_error(title, body);
        self.kick_me(delay_secs);
    }

    // ── Generic readers (delegated via HostApi) ─────────────────────────

    fn read_i32(&self, field: u32) -> i32 {
        unsafe { (self.host.player_read_i32)(self.id, field) }
    }
    fn read_f32(&self, field: u32) -> f32 {
        unsafe { (self.host.player_read_f32)(self.id, field) }
    }
    fn read_bool(&self, field: u32) -> bool {
        unsafe { (self.host.player_read_bool)(self.id, field) }
    }
    fn read_vec2(&self, field: u32) -> [f32; 2] {
        let mut out = [0.0f32; 2];
        unsafe { (self.host.player_read_vec2)(self.id, field, out.as_mut_ptr()) };
        out
    }
    fn read_vec3(&self, field: u32) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        unsafe { (self.host.player_read_vec3)(self.id, field, out.as_mut_ptr()) };
        out
    }
    fn read_string(&self, field: u32) -> String {
        read_string_via(|buf, cap| unsafe {
            (self.host.player_read_string)(self.id, field, buf, cap)
        })
    }
}

impl<'ctx> std::fmt::Debug for Player<'ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player")
            .field("id", &format_args!("{:#x}", self.id))
            .field("name", &self.name())
            .finish_non_exhaustive()
    }
}

/// Drives the `out_buf, cap -> required` calling convention used by
/// every host string getter. We try with a 256-byte stack buffer first
/// (covers most names); if the host reports a longer required size we
/// allocate and call again.
///
/// 推动所有宿主字符串 getter 通用的「out_buf, cap → required_len」
/// 调用约定。先用 256 字节栈缓冲区尝试一次（足够大多数名字），若
/// 宿主报告所需长度更大，再分配后重试一次。
pub(crate) fn read_string_via<F>(mut call: F) -> String
where
    F: FnMut(*mut c_char, usize) -> usize,
{
    let mut stack = [0u8; 256];
    let need = call(stack.as_mut_ptr() as *mut c_char, stack.len());
    if need == 0 {
        return String::new();
    }
    if need <= stack.len() {
        return cstr_to_string(&stack[..need]);
    }
    let mut heap = vec![0u8; need];
    let need2 = call(heap.as_mut_ptr() as *mut c_char, heap.len());
    let n = need2.min(heap.len());
    cstr_to_string(&heap[..n])
}

fn cstr_to_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
