//! Polyfield plugin SDK.
//!
//! Polyfield 插件 SDK。
//!
//! A plugin is a `cdylib` crate that depends on `polyfield` (this crate),
//! implements [`Plugin`] on a type, and exports the ABI entry via
//! [`declare_plugin!`]. Drop the built `.so` into the framework's
//! `plugins/` directory — it gets picked up on the next game launch.
//!
//! 一个插件是一个 `cdylib` crate，依赖本 `polyfield` crate，在某个类型
//! 上实现 [`Plugin`] trait，并通过 [`declare_plugin!`] 宏导出 ABI 入口。
//! 把编译出来的 `.so` 丢到框架的 `plugins/` 目录下，下次游戏启动时
//! 会被自动加载。
//!
//! # Quick start / 快速上手
//!
//! ```ignore
//! use polyfield::{Plugin, Ctx, PluginManifest, manifest, declare_plugin};
//! use polyfield::events::PlayerJoinEvent;
//!
//! #[derive(Default)]
//! struct Greeter;
//!
//! impl Plugin for Greeter {
//!     fn manifest() -> &'static PluginManifest {
//!         manifest!(
//!             name = "greeter", version = "0.1.0",
//!             authors = "me", description = "logs joiners",
//!         )
//!     }
//!
//!     fn on_player_join(&mut self, evt: &PlayerJoinEvent, ctx: &Ctx) {
//!         ctx.log_info(&format!("welcome {}", evt.name));
//!     }
//! }
//!
//! declare_plugin!(Greeter::default());
//! ```
//!
//! # What to read next / 继续阅读
//!
//! - [`Plugin`] — the trait you implement / 你要实现的 trait
//! - [`Ctx`] — host actions available inside callbacks / 回调中可用的宿主动作
//! - [`events`] — event payload definitions / 各事件的 payload 定义
//! - [`manifest!`] / [`declare_plugin!`] — the two macros you need to wire it up / 接入所需的两个宏

pub mod events;
pub mod fields;
pub mod game_enums;
mod abi;
mod context;
mod math;
mod player;
mod plugin;
mod vehicle;

pub use abi::{HostApi, LogLevel, PluginVTable, POLYFIELD_ABI_VERSION, POLYFIELD_ENTRY_SYMBOL};
pub use context::{Ctx, PlayerSnapshot};
pub use math::Vec3;
pub use player::Player;
pub use plugin::{Plugin, PluginManifest};
pub use vehicle::Vehicle;

#[doc(hidden)]
pub use abi::__build_vtable;

/// Wrap `msg` in a `<color=...>...</color>` tag for the in-game chat.
///
/// 把 `msg` 用 `<color=...>...</color>` 包起来，用于游戏内聊天染色。
///
/// `color` accepts any token the game's TextMeshPro / rich-text parser
/// understands: named colours (`"red"`, `"blue"`, `"green"`, ...) and
/// hex strings (`"#ff0080"`, `"#ff0080ff"`).
///
/// `color` 接受游戏内 TextMeshPro / 富文本解析器支持的任意写法：颜色名
/// （`"red"`、`"blue"`、`"green"` 等）或 hex 串（`"#ff0080"`、`"#ff0080ff"`）。
///
/// # Example
///
/// ```ignore
/// let line = format!("warning: {}", polyfield::color("red", "speed hack"));
/// ctx.host_say(&line);
/// ```
pub fn color(color: &str, msg: impl std::fmt::Display) -> String {
    format!("<color={color}>{msg}</color>")
}

/// Build a static [`PluginManifest`] inline.
///
/// 就地构造一个 `'static` 的 [`PluginManifest`]。
///
/// The macro stamps [`POLYFIELD_ABI_VERSION`] from the SDK build into
/// the manifest's `api_version` field so the loader can reject plugins
/// built against a mismatched SDK.
///
/// 宏会把编译时的 [`POLYFIELD_ABI_VERSION`] 自动写入 manifest 的
/// `api_version` 字段，loader 据此拒绝与当前 SDK 不匹配的插件。
///
/// # Example
///
/// ```ignore
/// fn manifest() -> &'static PluginManifest {
///     manifest!(
///         name = "anti-teleport", version = "0.1.0",
///         authors = "me", description = "flag impossible moves",
///     )
/// }
/// ```
#[macro_export]
macro_rules! manifest {
    (
        name = $name:expr,
        version = $version:expr,
        authors = $authors:expr,
        description = $description:expr $(,)?
    ) => {{
        static M: $crate::PluginManifest = $crate::PluginManifest {
            name: $name,
            version: $version,
            authors: $authors,
            description: $description,
            api_version: $crate::POLYFIELD_ABI_VERSION,
        };
        &M
    }};
}

/// Exports the ABI entry point the framework looks up via `dlsym`.
///
/// 导出框架通过 `dlsym` 查找的 ABI 入口符号。
///
/// `$ctor` is any expression that evaluates to a value implementing
/// [`Plugin`]. Common patterns:
///
/// ```ignore
/// declare_plugin!(MyPlugin { field: 0 });       // struct literal
/// declare_plugin!(MyPlugin::new());             // constructor call
/// ```
///
/// `$ctor` 是任意求值为 [`Plugin`] 实现者的表达式。常见写法：
///
/// ```ignore
/// declare_plugin!(MyPlugin { field: 0 });       // 结构体字面量
/// declare_plugin!(MyPlugin::new());             // 构造函数调用
/// ```
///
/// Call this **once** at the top of your plugin crate — the resulting
/// `polyfield_plugin_entry` is the only symbol the framework ever
/// looks up.
///
/// 在插件 crate 顶层调用**一次**即可——生成的 `polyfield_plugin_entry`
/// 是框架唯一会查找的符号。
#[macro_export]
macro_rules! declare_plugin {
    ($ctor:expr) => {
        #[no_mangle]
        pub extern "C" fn polyfield_plugin_entry(
            _host: &'static $crate::HostApi,
        ) -> $crate::PluginVTable {
            $crate::__build_vtable($ctor)
        }
    };
}
