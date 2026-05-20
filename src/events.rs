//! Event payloads passed from framework collectors to plugins.
//!
//! 由框架的 collector 推送给插件的事件 payload 集合。
//!
//! Keep these `#[repr(C)]`-friendly shapes and without lifetimes — events
//! cross the cdylib boundary.
//!
//! 所有类型都要保持对 `#[repr(C)]` 友好、不持有生命周期 —— 事件会跨
//! cdylib 边界传递。

use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::game_enums::{DamageType, GadgetId, VehicleType, WeaponId};
use crate::player::Player;
use crate::vehicle::Vehicle;
/// Opaque reference to a player instance.
///
/// 玩家实例的不透明引用。
///
/// This is a lightweight key (currently a `u64` derived from the
/// `PlayerControl` instance pointer). To access player fields or
/// perform actions, convert it to a [`Player`] handle via
/// `ctx.player(ref)` or the event's `.player(ctx)` / `.attacker(ctx)`
/// helpers.
///
/// 这是一个轻量级键（当前取自 `PlayerControl` 实例指针转成的 `u64`）。
/// 要访问玩家字段或执行动作，需通过 `ctx.player(ref)` 或事件上的
/// `.player(ctx)` / `.attacker(ctx)` 等 helper 将其转为 [`Player`] 句柄。
///
/// Can be used as a `HashMap` key for cross-event tracking.
///
/// 可用作 `HashMap` 的键来做跨事件追踪。
pub type PlayerRef = u64;

/// Fired the first time the framework sees a given player, and again when
/// that player renames themselves.
///
/// 首次观测到某位玩家时触发；玩家改名时再次触发。
///
/// Source: hooks `PlayerControl.RpcUpdateName(System.String)`. Mirror
/// broadcasts that RPC to every client on join and on rename; the
/// collector deduplicates so plugins see one event per actual change.
///
/// 事件来源：Hook 了 `PlayerControl.RpcUpdateName(System.String)`。
/// Mirror 会在玩家加入或改名时把该 RPC 广播给所有客户端；collector
/// 内部做了去重，所以插件每次真实改名只会收到一次。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerJoinEvent {
    /// Stable key for this player during the current session.
    ///
    /// 本局游戏内这位玩家的稳定键。
    pub player: PlayerRef,

    /// The name Mirror just announced. May be empty if the incoming
    /// string was null or failed UTF-16 decoding.
    ///
    /// Mirror 刚刚广播的名字。如果入参为 null 或 UTF-16 解码失败，
    /// 这里可能是空串。
    pub name: String,
}

impl PlayerJoinEvent {
    /// Build a high-level [`Player`] handle for the joining player.
    pub fn player<'c>(&self, ctx: &'c Ctx) -> Player<'c> {
        ctx.player(self.player)
    }
}

/// A player dealt damage to another entity.
///
/// 一位玩家对另一个实体造成了伤害。
///
/// Source: hooks `PlayerControl.RpcDamageEntities`. The `this` pointer
/// is the **attacker**; `_target` is the victim's `PlayerControl`
/// instance pointer (or 0 if the target is an NPC / non-player).
///
/// 事件来源：Hook 了 `PlayerControl.RpcDamageEntities`。`this` 是
/// **攻击者**；`_target` 是受害者的 `PlayerControl` 实例指针（目标为
/// NPC / 非玩家时为 0）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct  DamageEvent {
    /// The player who initiated the damage (the `this` of the RPC).
    /// `0` when the damage came from a non-player source (e.g. grenade
    /// explosion arbitration, server-side artillery) — guard with
    /// `attacker != 0` before treating it as a real player.
    ///
    /// 发起伤害的玩家（RPC 的 `this`）。当伤害来自非玩家来源（手雷
    /// 爆炸仲裁、服务器端炮击等）时为 `0`，使用前需判断
    /// `attacker != 0` 是否为真实玩家。
    pub attacker: PlayerRef,

    /// The target's Mirror netId. `0` when the target is an NPC or
    /// couldn't be resolved. **Not** a `PlayerControl` pointer — to get
    /// a `Player` handle, call [`Self::victim`] which performs the
    /// netId → player lookup for you.
    ///
    /// 目标的 Mirror netId。目标为 NPC 或无法解析时为 `0`。**不是**
    /// `PlayerControl` 指针——需要 `Player` 句柄请用 [`Self::victim`]，
    /// 它会做 netId 到 PlayerControl 的查找。
    pub victim: PlayerRef,

    /// Damage amount (integer in the game's RPC signature).
    ///
    /// 伤害数值（游戏 RPC 签名中为整数）。
    pub amount: i32,

    /// Damage type enum value from the game.
    ///
    /// 游戏中的伤害类型枚举值。
    pub damage_type: i32,

    /// Weapon identifier.
    ///
    /// 武器标识。
    pub weapon_id: i32,

    /// `true` if the target is an NPC.
    ///
    /// 目标为 NPC 时为 `true`。
    pub is_npc: bool,

    /// Extra data string attached to the RPC. May be empty.
    ///
    /// RPC 附带的额外数据字符串，可能为空。
    pub data: String,

    /// Monotonic frame counter, `0` if not yet wired.
    ///
    /// 单调递增的帧号；尚未接入时为 `0`。
    pub frame: u64,
}

impl DamageEvent {
    /// Build a [`Player`] handle for the attacker.
    pub fn attacker<'c>(&self, ctx: &'c Ctx) -> Player<'c> {
        ctx.player(self.attacker)
    }

    /// Build a [`Player`] handle for the victim. Returns `None` when
    /// the target is an NPC, unknown (`victim == 0`), or its netId
    /// can't be resolved to a `PlayerControl`.
    pub fn victim<'c>(&self, _ctx: &'c Ctx) -> Option<Player<'c>> {
        None
    }

    /// Typed damage type. `None` if the game sent an unknown value.
    ///
    /// 带类型的伤害类型。游戏发送未知值时返回 `None`。
    pub fn damage_type_enum(&self) -> Option<DamageType> {
        DamageType::from_raw(self.damage_type)
    }

    /// Typed weapon id (primary/secondary firearms). `None` if unknown.
    ///
    /// 带类型的武器 ID（主/副武器）。未知时返回 `None`。
    pub fn weapon_enum(&self) -> Option<WeaponId> {
        WeaponId::from_raw(self.weapon_id)
    }

    /// Typed gadget id (equipment slot). `None` if unknown.
    ///
    /// 带类型的道具 ID（装备栏）。未知时返回 `None`。
    pub fn gadget_enum(&self) -> Option<GadgetId> {
        GadgetId::from_raw(self.weapon_id)
    }
}

/// A latency measurement for a player.
///
/// 某位玩家的网络延迟测量。
///
/// Source: hooks `PlayerControl.RpcUpdateLatency`. Fires roughly once
/// per server tick per player.
///
/// 事件来源：Hook 了 `PlayerControl.RpcUpdateLatency`。大约每个服务器
/// tick 每位玩家触发一次。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencySample {
    pub player: PlayerRef,

    /// Round-trip delay in milliseconds as reported by the server.
    ///
    /// 服务器上报的往返延迟，单位毫秒。
    pub ms: f32,
}

impl LatencySample {
    /// Build a [`Player`] handle for the sampled player.
    ///
    /// 为被采样的玩家构造 [`Player`] 句柄。
    pub fn player<'c>(&self, ctx: &'c Ctx) -> Player<'c> {
        ctx.player(self.player)
    }
}

/// Low-frequency tick. Useful for periodic housekeeping that should
/// happen roughly once per frame rather than inside a hot hook.
///
/// 低频 tick 事件。适合做「每帧大致一次」的周期性维护，不建议在 hook
/// 热路径里做的工作放这里。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickEvent {
    pub frame: u64,
}

/// Fires after `GameManager.Start` runs — i.e., immediately after a new
/// game/match begins. **Notification only** (cannot be intercepted —
/// blocking would prevent the new game from initialising).
///
/// 在 `GameManager.Start` 跑完之后触发——即新一局/比赛刚开始时。
/// **仅通知**，不可拦截（拦截会阻止新一局正常初始化）。
///
/// Use this to reset per-game state. Query the new map / match type via
/// [`crate::Ctx::game_map`] and [`crate::Ctx::match_type`] — both are
/// freshly available by the time this event fires.
///
/// 用于重置每局状态。新地图与 match type 通过 [`crate::Ctx::game_map`]
/// 和 [`crate::Ctx::match_type`] 查询——事件触发时这两个值已就绪。
///
/// `frame` is the tick collector's current frame counter at emit time
/// (`0` if the tick collector hasn't started yet).
///
/// `frame` 是触发时刻 tick collector 的当前帧号（tick collector 尚未
/// 启动时为 `0`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStartEvent {
    pub frame: u64,
}

/// A player sent a chat message. **Interceptable**: the handler returns
/// `bool` (`true` to forward, `false` to swallow) and may modify the
/// message before it's forwarded.
///
/// 玩家发送了一条聊天消息。**可拦截**：处理函数返回 `bool`
/// （`true` 放行，`false` 吞掉），并可在放行前修改消息内容。
///
/// Source: hooks `PlayerControl.UserCode_CmdSendChat__String`.
///
/// 事件来源：Hook 了 `PlayerControl.UserCode_CmdSendChat__String`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEvent {
    /// The player who sent the message.
    ///
    /// 发送消息的玩家。
    pub sender: PlayerRef,

    /// The chat message content. Plugins may **modify** this field;
    /// the framework will use the final value when forwarding the call
    /// (if not blocked).
    ///
    /// 聊天消息内容。插件可以**修改**此字段；框架在放行时会使用
    /// 最终值调用原始函数。
    pub message: String,
}

impl ChatEvent {
    /// Build a [`Player`] handle for the sender.
    pub fn sender<'c>(&self, ctx: &'c Ctx) -> Player<'c> {
        ctx.player(self.sender)
    }
}

/// A player respawned. **Interceptable.**
///
/// 玩家重生。**可拦截。**
///
/// Source: hooks
/// `PlayerControl.UserCode_CmdRespawn__String__UInt32(string, uint)`.
/// `this` is the player respawning.
///
/// 事件来源：Hook 了
/// `PlayerControl.UserCode_CmdRespawn__String__UInt32(string, uint)`。
/// `this` 即重生玩家。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespawnEvent {
    pub player: PlayerRef,

    /// Spawn data string. **Mutable** — modify before returning `true`
    /// to forward with new contents.
    ///
    /// 重生数据字符串。**可变**——在返回 `true` 前修改即可改写转发内容。
    pub spawn_data: String,

    /// Vehicle type id.
    ///
    /// 载具类型 id。
    pub vehicle_type: u32,
}

impl RespawnEvent {
    pub fn player<'c>(&self, ctx: &'c Ctx) -> Player<'c> {
        ctx.player(self.player)
    }
}

/// A player threw a grenade. **Interceptable.**
///
/// 玩家投掷了手雷。**可拦截。**
///
/// Source: hooks `PlayerCombat.UserCode_CmdGrenade__String(string)`.
/// `player` is resolved via `PlayerCombat.playerControl` so the ref
/// matches what other events use.
///
/// 事件来源：Hook 了 `PlayerCombat.UserCode_CmdGrenade__String(string)`。
/// `player` 通过 `PlayerCombat.playerControl` 解出，与其它事件一致。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrenadeEvent {
    pub player: PlayerRef,

    /// Grenade data string. **Mutable** — modify before returning `true`
    /// to forward with new contents.
    ///
    /// 手雷数据字符串。**可变**。
    pub grenade_data: String,
}

impl GrenadeEvent {
    pub fn player<'c>(&self, ctx: &'c Ctx) -> Player<'c> {
        ctx.player(self.player)
    }
}

/// A player fired their weapon. **Interceptable.**
///
/// 玩家开火。**可拦截。**
///
/// Source: hooks
/// `PlayerCombat.UserCode_CmdShoot__Byte__String(byte, string)`.
/// `player` is resolved via `PlayerCombat.playerControl` so the ref
/// matches what other events use.
///
/// 事件来源：Hook 了
/// `PlayerCombat.UserCode_CmdShoot__Byte__String(byte, string)`。
/// `player` 通过 `PlayerCombat.playerControl` 解出，与其它事件一致。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShootEvent {
    pub player: PlayerRef,

    /// Weapon type id.
    ///
    /// 武器类型 id。
    pub weapon_type: u8,

    /// Shoot data string (trajectory / target info — game-specific).
    /// **Mutable** — modify before returning `true` to forward with
    /// new contents.
    ///
    /// 开火数据字符串（弹道/目标等，游戏内部约定）。**可变**。
    pub shoot_data: String,
}

impl ShootEvent {
    pub fn player<'c>(&self, ctx: &'c Ctx) -> Player<'c> {
        ctx.player(self.player)
    }

    /// Typed weapon id (primary/secondary firearms). `None` if unknown.
    pub fn weapon_enum(&self) -> Option<WeaponId> {
        WeaponId::from_raw(self.weapon_type as i32)
    }
}

/// A player started reloading. **Notification only** — cannot be
/// intercepted (blocking the animation RPC would desync clients).
///
/// 玩家开始换弹。**仅通知**——不可拦截（拦截动画 RPC 会导致客户端
/// 状态不同步）。
///
/// Source: hooks `PlayerControl.RpcCallAnimation(string)` and filters
/// for calls where the animation name contains `"Reloading"`.
///
/// 事件来源：Hook `PlayerControl.RpcCallAnimation(string)`，仅当动画
/// 名包含 `"Reloading"` 时触发。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadEvent {
    pub player: PlayerRef,

    /// The full animation name string (e.g. `"Reloading"`,
    /// `"ReloadingBolt"`).
    ///
    /// 完整的动画名字符串。
    pub anim_name: String,
}

impl ReloadEvent {
    pub fn player<'c>(&self, ctx: &'c Ctx) -> Player<'c> {
        ctx.player(self.player)
    }
}

/// A player fired a vehicle weapon. **Interceptable.**
///
/// 玩家在载具中开火。**可拦截。**
///
/// Source: hooks `PlayerVehicle.CmdVehicleShoot(uint, int)`.
/// `player` is resolved via `PlayerVehicle.playerControl`.
///
/// 事件来源：Hook `PlayerVehicle.CmdVehicleShoot(uint, int)`。
/// `player` 通过 `PlayerVehicle.playerControl` 解出。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleShootEvent {
    pub player: PlayerRef,

    /// Pointer to the `VehicleControl` instance (as u64). Use with
    /// `ctx.vehicle(evt.vehicle)` once vehicle handles are available,
    /// or read fields manually via the bridge.
    ///
    /// `VehicleControl` 实例指针。
    pub vehicle: PlayerRef,

    /// The vehicle's Mirror netId.
    pub vehicle_id: u32,

    /// The seat the shooter occupies (0 = driver, 1+ = gunner seats).
    pub seat_id: i32,
}

impl VehicleShootEvent {
    pub fn player<'c>(&self, ctx: &'c Ctx) -> Player<'c> {
        ctx.player(self.player)
    }

    /// Get the `Vehicle` handle for the vehicle that fired.
    pub fn vehicle<'c>(&self, ctx: &'c Ctx) -> Vehicle<'c> {
        ctx.vehicle(self.vehicle)
    }
}

/// A vehicle is being repaired. **Interceptable.**
///
/// 载具正在被修理。**可拦截。**
///
/// Source: hooks `PlayerVehicle.RpcVehicleRepair(uint, int, int)`.
/// `player` is resolved via `PlayerVehicle.playerControl`.
///
/// 事件来源：Hook `PlayerVehicle.RpcVehicleRepair(uint, int, int)`。
/// `player` 通过 `PlayerVehicle.playerControl` 解出。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleRepairEvent {
    pub player: PlayerRef,

    /// The vehicle's Mirror netId.
    pub vehicle_id: u32,

    /// Repair timer value.
    pub timer: i32,

    /// Health value after repair.
    pub health: i32,
}

impl VehicleRepairEvent {
    pub fn player<'c>(&self, ctx: &'c Ctx) -> Player<'c> {
        ctx.player(self.player)
    }
}
