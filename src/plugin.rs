//! The Plugin trait is the surface area plugin authors implement.
//!
//! 插件作者需要实现的唯一 trait 及其元数据类型。

use crate::context::Ctx;
use crate::events::{
    ChatEvent, DamageEvent, GameStartEvent, GrenadeEvent, LatencySample, PlayerJoinEvent,
    ReloadEvent, RespawnEvent, ShootEvent, TickEvent, VehicleRepairEvent, VehicleShootEvent,
};

/// Static plugin metadata surfaced in logs and on the management panel.
///
/// 插件的静态元数据，会在日志和后续管理面板上展示。
///
/// Build one with the [`crate::manifest!`] macro rather than constructing
/// the struct directly — the macro stamps [`api_version`] from the SDK
/// build, which the framework checks at load time.
///
/// 请用 [`crate::manifest!`] 宏来构造，而不是手写这个结构体。宏会把
/// 当前 SDK 的 [`api_version`] 一起嵌进来，框架在加载时会校验该值。
///
/// [`api_version`]: PluginManifest::api_version
#[repr(C)]
pub struct PluginManifest {
    /// Short identifier, shown in log prefixes like `[my-plugin]`.
    ///
    /// 简短标识，在日志前缀中显示，如 `[my-plugin]`。
    pub name: &'static str,

    /// Human-readable version string. No semver enforcement on the
    /// framework side.
    ///
    /// 人类可读的版本字符串。框架不会做 semver 校验。
    pub version: &'static str,

    /// Free-form author string. Single author or team name, whatever
    /// you prefer.
    ///
    /// 任意格式的作者字段。单个作者或团队名，按喜好填写即可。
    pub authors: &'static str,

    /// One-line description used in startup logs and the future
    /// management panel.
    ///
    /// 一行描述，用于启动日志和后续的管理面板。
    pub description: &'static str,

    /// Must equal [`crate::POLYFIELD_ABI_VERSION`] — the framework refuses
    /// to load plugins built against a mismatched SDK.
    ///
    /// 必须等于 [`crate::POLYFIELD_ABI_VERSION`]。框架加载时若 ABI
    /// 版本不匹配会直接拒绝该插件。通常你不需要手填 —— 用
    /// [`crate::manifest!`] 宏就会自动带上。
    pub api_version: u32,
}

/// The trait every plugin implements.
///
/// 每个插件都需要实现的 trait。
///
/// All event callbacks have empty default implementations, so plugins
/// only override the ones they actually care about. The `Ctx` handle
/// exposes host actions (kick/ban/quit/…) and read-only state queries.
///
/// 所有事件回调都有空的默认实现，插件只重写自己关心的即可。
/// 每次回调都会收到 `Ctx` 句柄，可用来调用宿主动作（踢出/封禁/
/// 强退等）或做只读状态查询。
///
/// # Examples
///
/// ```ignore
/// use polyfield::{Plugin, Ctx, PluginManifest, manifest, declare_plugin};
/// use polyfield::events::PlayerJoinEvent;
///
/// #[derive(Default)]
/// struct Greeter;
///
/// impl Plugin for Greeter {
///     fn manifest() -> &'static PluginManifest {
///         manifest!(name = "greeter", version = "0.1.0",
///                   authors = "me", description = "logs joiners")
///     }
///
///     fn on_player_join(&mut self, evt: &PlayerJoinEvent, ctx: &Ctx) {
///         ctx.log_info(&format!("welcome {}", evt.name));
///     }
/// }
///
/// declare_plugin!(Greeter::default());
/// ```
pub trait Plugin: Send + Sync + 'static {
    /// Return a reference to this plugin's static manifest. The returned
    /// reference must be `'static` — typically built in place by the
    /// [`crate::manifest!`] macro.
    ///
    /// 返回该插件的静态 manifest。返回值必须是 `'static`，通常由
    /// [`crate::manifest!`] 宏就地构造。
    fn manifest() -> &'static PluginManifest
    where
        Self: Sized;

    /// Called once after the plugin is loaded and registered, before
    /// any event fires. Good place for one-time setup and a "we're
    /// online" log line.
    ///
    /// 插件加载并注册完成后、任何事件触发之前调用一次。适合在这里
    /// 做一次性初始化，或写一条「已上线」的日志。
    fn on_load(&mut self, _ctx: &Ctx) {}

    /// Called once during framework shutdown, after which the plugin's
    /// `.so` will be unloaded. Currently not invoked in v1 — the
    /// process typically exits without a clean teardown path.
    ///
    /// 框架关闭时调用一次，之后插件的 `.so` 会被卸载。v1 暂时不会
    /// 被实际触发——游戏进程通常直接退出，没有优雅停机路径。
    fn on_unload(&mut self, _ctx: &Ctx) {}

    /// A player was seen for the first time, or renamed.
    /// **Notification only.** See [`PlayerJoinEvent`] for dedup semantics.
    ///
    /// 首次观测到某玩家或玩家改名时触发。**仅通知。** 去重细节见
    /// [`PlayerJoinEvent`]。
    ///
    /// Observe-only: the name broadcast can't be blocked, so this hook
    /// takes `&PlayerJoinEvent` and returns nothing.
    ///
    /// 仅观测：名字广播无法阻止，因此该回调接收 `&PlayerJoinEvent`
    /// 且无返回值。
    fn on_player_join(&mut self, _evt: &PlayerJoinEvent, _ctx: &Ctx) {}

    /// A player dealt damage. **Interceptable.** See [`DamageEvent`].
    ///
    /// 玩家造成了伤害。**可拦截。** 详见 [`DamageEvent`]。
    ///
    /// Return `true` to forward (with possibly modified parameters),
    /// `false` to swallow the damage RPC entirely. Default: `true`.
    ///
    /// 返回 `true` 放行（可使用修改过的参数），返回 `false` 完全
    /// 吞掉这次伤害 RPC。默认 `true`。
    fn on_damage(&mut self, _evt: &mut DamageEvent, _ctx: &Ctx) -> bool {
        true
    }

    /// New latency measurement for a player. See [`LatencySample`].
    ///
    /// 某玩家的延迟测量更新。详见 [`LatencySample`]。
    ///
    /// **Note:** there is no `on_move`. Movement data is best polled
    /// inside `on_tick` via `Player::position()` — that reads the
    /// authoritative `_netTransform._recivedPos` and lets each plugin
    /// pick its own sampling cadence without framework-wide overhead.
    ///
    /// **注意：** 没有 `on_move`。移动数据在 `on_tick` 里通过
    /// `Player::position()` 轮询最合适——它读取权威的
    /// `_netTransform._recivedPos`，且允许每个插件自选采样节奏，
    /// 不会给框架增加全局开销。
    fn on_latency(&mut self, _evt: &LatencySample, _ctx: &Ctx) {}

    /// Low-frequency timer tick. See [`TickEvent`].
    ///
    /// 低频定时事件。详见 [`TickEvent`]。
    fn on_tick(&mut self, _evt: &TickEvent, _ctx: &Ctx) {}

    /// A player sent a chat message. **Interceptable.**
    ///
    /// 玩家发送了聊天消息。**可拦截。**
    ///
    /// Return `true` to forward (with possibly modified `evt.message`),
    /// `false` to swallow the message. Default: `true`.
    ///
    /// 返回 `true` 放行（可使用修改过的 `evt.message`），返回 `false`
    /// 吞掉消息。默认 `true`。
    fn on_chat(&mut self, _evt: &mut ChatEvent, _ctx: &Ctx) -> bool {
        true
    }

    /// A new game/match has just started — fires after `GameManager.Start`
    /// runs. Notification only. Use this to reset per-game state.
    ///
    /// 新一局/比赛刚开始——在 `GameManager.Start` 跑完之后触发。
    /// 仅通知。用于重置每局状态。
    ///
    /// Query the fresh map / match type via [`Ctx::game_map`] /
    /// [`Ctx::match_type`].
    ///
    /// 通过 [`Ctx::game_map`] / [`Ctx::match_type`] 读取新一局的地图与
    /// match type。
    fn on_game_start(&mut self, _evt: &GameStartEvent, _ctx: &Ctx) {}

    /// A player respawned. **Interceptable.** See [`RespawnEvent`].
    ///
    /// 玩家重生。**可拦截。** 详见 [`RespawnEvent`]。
    ///
    /// Return `true` to forward (with possibly modified params),
    /// `false` to swallow the respawn entirely. Default: `true`.
    ///
    /// 返回 `true` 放行（可使用修改过的参数），返回 `false` 完全
    /// 阻止此次重生。默认 `true`。
    fn on_respawn(&mut self, _evt: &mut RespawnEvent, _ctx: &Ctx) -> bool {
        true
    }

    /// A player threw a grenade. **Interceptable.** See [`GrenadeEvent`].
    ///
    /// 玩家投掷手雷。**可拦截。** 详见 [`GrenadeEvent`]。
    fn on_grenade(&mut self, _evt: &mut GrenadeEvent, _ctx: &Ctx) -> bool {
        true
    }

    /// A player fired their weapon. **Interceptable.** See [`ShootEvent`].
    ///
    /// 玩家开火。**可拦截。** 详见 [`ShootEvent`]。
    fn on_shoot(&mut self, _evt: &mut ShootEvent, _ctx: &Ctx) -> bool {
        true
    }

    /// A player started reloading. **Notification only.**
    /// See [`ReloadEvent`].
    ///
    /// 玩家开始换弹。**仅通知。** 详见 [`ReloadEvent`]。
    fn on_reload(&mut self, _evt: &ReloadEvent, _ctx: &Ctx) {}

    /// A player fired a vehicle weapon. **Interceptable.**
    /// See [`VehicleShootEvent`].
    ///
    /// 玩家在载具中开火。**可拦截。** 详见 [`VehicleShootEvent`]。
    fn on_vehicle_shoot(&mut self, _evt: &mut VehicleShootEvent, _ctx: &Ctx) -> bool {
        true
    }

    /// A vehicle is being repaired. **Interceptable.**
    /// See [`VehicleRepairEvent`].
    ///
    /// 载具正在被修理。**可拦截。** 详见 [`VehicleRepairEvent`]。
    fn on_vehicle_repair(&mut self, _evt: &mut VehicleRepairEvent, _ctx: &Ctx) -> bool {
        true
    }
}
