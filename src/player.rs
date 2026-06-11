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
use crate::game_enums::{ClassRole, UserState, WeaponId};

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
    /// `myState` — raw `UserState` integer. See [`user_state`](Self::user_state)
    /// for the typed view.
    pub fn user_state_raw(&self) -> i32 { self.read_i32(f::F_USER_STATE) }
    /// `myState` as a typed [`UserState`]. `None` if the game sent an
    /// unknown value.
    ///
    /// `myState` 的类型化 [`UserState`]。游戏发送未知值时返回 `None`。
    pub fn user_state(&self) -> Option<UserState> {
        UserState::from_raw(self.user_state_raw())
    }
    /// `myClass` — raw `ClassRole` integer. See [`class_role`](Self::class_role)
    /// for the typed view.
    pub fn class_role_raw(&self) -> i32 { self.read_i32(f::F_CLASS_ROLE) }
    /// `myClass` as a typed [`ClassRole`]. `None` if the game sent an
    /// unknown value.
    ///
    /// `myClass` 的类型化 [`ClassRole`]。游戏发送未知值时返回 `None`。
    pub fn class_role(&self) -> Option<ClassRole> {
        ClassRole::from_raw(self.class_role_raw())
    }

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

    /// Kill/death ratio. Returns the kill count when the player hasn't
    /// died yet (avoids division by zero), `0.0` when both are zero.
    ///
    /// 击杀/死亡比。玩家尚未死亡时返回击杀数（避免除零），两者都为
    /// 零时返回 `0.0`。
    pub fn kdr(&self) -> f32 {
        let deaths = self.death_count();
        if deaths <= 0 {
            self.kill_count() as f32
        } else {
            self.kill_count() as f32 / deaths as f32
        }
    }

    // ── Movement / pose ─────────────────────────────────────────────────

    /// `playerSpeed` — the player's **configured move-speed constant**, a
    /// static stat set from the class/loadout. **This is NOT instantaneous
    /// velocity.** It does not change as the player moves, stops, or gets
    /// boosted, so it is useless for speed-hack detection.
    ///
    /// `playerSpeed` —— 玩家**配置的移动速度常量**，由兵种/装备写死的静态
    /// 属性。**不是实时速度。** 玩家移动、静止、加速都不会改变它，因此
    /// 对加速作弊检测毫无用处。
    ///
    /// For actual movement speed, take the magnitude of
    /// [`velocity`](Self::velocity) (or [`net_velocity`](Self::net_velocity)
    /// for the un-fallback'd value) — e.g. `Vec3::from(p.velocity()).magnitude_2d()`
    /// for ground speed. That reads the replicated `_recivedVel`.
    ///
    /// 实时移动速度请取 [`velocity`](Self::velocity) 的模长（需要未回退值
    /// 用 [`net_velocity`](Self::net_velocity)）——例如地面速度用
    /// `Vec3::from(p.velocity()).magnitude_2d()`，它读的是同步过来的
    /// `_recivedVel`。
    pub fn speed(&self) -> f32 { self.read_f32(f::F_PLAYER_SPEED) }
    pub fn is_running(&self) -> bool { self.read_bool(f::F_RUNNING) }
    pub fn is_grounded(&self) -> bool { self.read_bool(f::F_GROUNDED) }
    /// `crouch` — crouch state level (int).
    pub fn crouch(&self) -> i32 { self.read_i32(f::F_CROUCH) }
    pub fn is_under_water(&self) -> bool { self.read_bool(f::F_IS_UNDER_WATER) }

    /// World position, with a convenience fallback: returns
    /// `_netTransform._recivedPos`, or `lastPlayerPos` when the net
    /// transform reads as all-zero (typically not yet replicated).
    ///
    /// 世界坐标，带便利回退：返回 `_netTransform._recivedPos`，当 net
    /// transform 读出全零（通常是尚未同步）时回退到 `lastPlayerPos`。
    ///
    /// **Caveat for precise checks:** a player genuinely standing at the
    /// world origin reads as all-zero and triggers the fallback. If you
    /// need to distinguish "no data" from "really at origin" (e.g.
    /// teleport / speed detection), use [`net_position`](Self::net_position),
    /// which returns `None` instead of silently falling back.
    ///
    /// **精确判定注意：** 真站在世界原点的玩家也读出全零并触发回退。
    /// 若需区分「无数据」与「真在原点」（如瞬移/加速检测），改用
    /// [`net_position`](Self::net_position)，它返回 `None` 而不是静默回退。
    pub fn position(&self) -> [f32; 3] {
        self.net_position()
            .unwrap_or_else(|| self.read_vec3(f::F_LAST_PLAYER_POS))
    }

    /// Raw `_netTransform._recivedPos` without any fallback. `None` when
    /// it reads as all-zero — i.e. the net transform hasn't replicated a
    /// position yet. Prefer this in detection logic where the difference
    /// between "unavailable" and "at origin" matters.
    ///
    /// 原始 `_netTransform._recivedPos`，不做任何回退。读出全零时返回
    /// `None`（即 net transform 尚未同步位置）。在「不可用」与「在原点」
    /// 有区别的检测逻辑里优先用它。
    pub fn net_position(&self) -> Option<[f32; 3]> {
        let v = self.read_vec3(f::F_NET_POSITION);
        (v != [0.0, 0.0, 0.0]).then_some(v)
    }

    /// Velocity, with a convenience fallback: returns
    /// `_netTransform._recivedVel`, or `myRigidVel` when the net
    /// transform reads as all-zero. See [`position`](Self::position) for
    /// the all-zero caveat; use [`net_velocity`](Self::net_velocity) for
    /// the un-fallback'd value.
    ///
    /// 速度，带便利回退：返回 `_netTransform._recivedVel`，net transform
    /// 读出全零时回退到 `myRigidVel`。全零注意事项见
    /// [`position`](Self::position)；需要未回退值用
    /// [`net_velocity`](Self::net_velocity)。
    pub fn velocity(&self) -> [f32; 3] {
        self.net_velocity()
            .unwrap_or_else(|| self.read_vec3(f::F_MY_RIGID_VEL))
    }

    /// Raw `_netTransform._recivedVel` without fallback. `None` when it
    /// reads as all-zero.
    ///
    /// 原始 `_netTransform._recivedVel`，不回退。读出全零时返回 `None`。
    pub fn net_velocity(&self) -> Option<[f32; 3]> {
        let v = self.read_vec3(f::F_NET_VELOCITY);
        (v != [0.0, 0.0, 0.0]).then_some(v)
    }

    /// `_moveDir` — current movement direction vector.
    pub fn move_dir(&self) -> [f32; 3] { self.read_vec3(f::F_MOVE_DIR) }

    /// `_lookDir` — 2D look direction.
    pub fn look_dir(&self) -> [f32; 2] { self.read_vec2(f::F_LOOK_DIR) }

    /// [`position`](Self::position) as a [`Vec3`](crate::Vec3), for ergonomic
    /// distance math (`p.pos().distance_2d(other.pos())`).
    ///
    /// [`position`](Self::position) 的 [`Vec3`](crate::Vec3) 形式，方便做
    /// 距离运算（`p.pos().distance_2d(other.pos())`）。
    pub fn pos(&self) -> crate::Vec3 { self.position().into() }

    /// [`velocity`](Self::velocity) as a [`Vec3`](crate::Vec3). Pair with
    /// [`Vec3::magnitude_2d`](crate::Vec3::magnitude_2d) for ground speed.
    ///
    /// [`velocity`](Self::velocity) 的 [`Vec3`](crate::Vec3) 形式。配合
    /// [`Vec3::magnitude_2d`](crate::Vec3::magnitude_2d) 得地面速度。
    pub fn vel(&self) -> crate::Vec3 { self.velocity().into() }

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
    /// See [`weapon`](Self::weapon) for the typed view.
    ///
    /// 当前装备武器 ID（来自 `playerCombat.currWeaponID`）。类型化视图
    /// 见 [`weapon`](Self::weapon)。
    pub fn weapon_id(&self) -> i32 { self.read_i32(f::F_WEAPON_ID) }

    /// Currently equipped weapon as a typed [`WeaponId`]. `None` if the
    /// raw id doesn't map to a known weapon.
    ///
    /// 当前装备武器的类型化 [`WeaponId`]。原始 id 不对应已知武器时
    /// 返回 `None`。
    pub fn weapon(&self) -> Option<WeaponId> {
        WeaponId::from_raw(self.weapon_id())
    }

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
    ///
    /// No-op (logged) if this handle refers to the host player — the
    /// framework refuses to kill the server's own player.
    ///
    /// 若该句柄指向 host 玩家则为空操作(会记日志)——框架拒绝杀死
    /// 服务器自己的玩家。
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
    ///
    /// No-op (logged) if this handle refers to the host player.
    ///
    /// 若该句柄指向 host 玩家则为空操作(会记日志)。
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

    /// Send a private chat message from `sender` to this player only.
    /// Uses Mirror's targeted RPC — only this player's client receives it.
    /// For server-wide broadcast use [`Ctx::host_say`](crate::Ctx::host_say).
    /// Use [`color`](crate::color) to colourise.
    ///
    /// 从 `sender` 向**该玩家**发送私聊消息（定向 RPC，只有该玩家客户端
    /// 收到）。全服广播用 [`Ctx::host_say`](crate::Ctx::host_say)。染色用
    /// [`color`](crate::color)。
    pub fn send_chat_from(&self, sender: Player, msg: &str) {
        let c = CString::new(msg).unwrap_or_default();
        unsafe { (self.host.player_send_chat)(sender.id, self.id, c.as_ptr()) };
    }

    /// Force this player's display name via
    /// `PlayerControl.RpcUpdateName`. Useful for sanitising names (e.g.
    /// stripping rich-text exploits) or stamping a slot prefix.
    ///
    /// 通过 `PlayerControl.RpcUpdateName` 强制设置该玩家显示名。可用于
    /// 清洗名字（如剥离富文本利用）或打上槽位前缀。
    pub fn update_name(&self, name: &str) {
        let c = CString::new(name).unwrap_or_default();
        unsafe { (self.host.player_update_name)(self.id, c.as_ptr()) };
    }

    /// Trigger an animation on this player via
    /// `PlayerControl.RpcCallAnimation` (e.g. `"Reloading"`).
    ///
    /// 通过 `PlayerControl.RpcCallAnimation` 在该玩家身上触发动画
    /// （如 `"Reloading"`）。
    pub fn call_animation(&self, anim: &str) {
        let c = CString::new(anim).unwrap_or_default();
        unsafe { (self.host.player_call_animation)(self.id, c.as_ptr()) };
    }

    // ── Vehicle association ─────────────────────────────────────────────────

    /// The vehicle this player is currently in, or `None` when on foot.
    /// Mirrors [`Vehicle::driver`](crate::Vehicle::driver) in the other
    /// direction. Backed by `playerVehicle.currentVehicle` gated on
    /// `IsInVehicle()`.
    ///
    /// 该玩家当前所在的载具，未乘载具时为 `None`。是
    /// [`Vehicle::driver`](crate::Vehicle::driver) 的反方向。
    pub fn vehicle(&self) -> Option<crate::Vehicle<'ctx>> {
        let id = unsafe { (self.host.player_vehicle)(self.id) };
        (id != 0).then(|| crate::Vehicle::new(id, self.host))
    }

    /// `true` if this player is currently in a vehicle.
    ///
    /// 该玩家当前是否在载具中。
    pub fn is_in_vehicle(&self) -> bool {
        unsafe { (self.host.player_vehicle)(self.id) != 0 }
    }

    // ── Raw field escape hatch ──────────────────────────────────────────────

    /// Read an arbitrary `PlayerControl` field by its [`PlayerField`] id
    /// from [`crate::fields`], typed as `i32`. Escape hatch for fields
    /// that don't yet have a dedicated typed getter — the host still
    /// validates the id against its offset table, returning `0` for
    /// anything it doesn't recognise.
    ///
    /// 按 [`crate::fields`] 中的 [`PlayerField`] id 读取任意
    /// `PlayerControl` 字段（按 `i32` 解释）。这是尚无专用 getter 字段的
    /// 逃生舱——宿主仍会用偏移表校验 id，无法识别时返回 `0`。
    ///
    /// [`PlayerField`]: crate::fields::PlayerField
    pub fn read_raw_i32(&self, field: crate::fields::PlayerField) -> i32 {
        self.read_i32(field)
    }

    /// Read an arbitrary `PlayerControl` field as `f32`. See
    /// [`read_raw_i32`](Self::read_raw_i32).
    ///
    /// 按 `f32` 读取任意字段。详见 [`read_raw_i32`](Self::read_raw_i32)。
    pub fn read_raw_f32(&self, field: crate::fields::PlayerField) -> f32 {
        self.read_f32(field)
    }

    /// Read an arbitrary `PlayerControl` field as `bool`. See
    /// [`read_raw_i32`](Self::read_raw_i32).
    ///
    /// 按 `bool` 读取任意字段。详见 [`read_raw_i32`](Self::read_raw_i32)。
    pub fn read_raw_bool(&self, field: crate::fields::PlayerField) -> bool {
        self.read_bool(field)
    }

    /// Read an arbitrary `PlayerControl` field as a `Vec3`. See
    /// [`read_raw_i32`](Self::read_raw_i32).
    ///
    /// 按 `Vec3` 读取任意字段。详见 [`read_raw_i32`](Self::read_raw_i32)。
    pub fn read_raw_vec3(&self, field: crate::fields::PlayerField) -> [f32; 3] {
        self.read_vec3(field)
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
