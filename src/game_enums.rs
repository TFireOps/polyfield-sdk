//! Game-specific enumerations.
//!
//! 游戏特定枚举。
//!
//! These map the raw integer IDs used in RPCs and fields to
//! human-readable Rust enum variants. Plugin authors can pattern-match
//! on them directly; the raw `i32` fields on events remain available
//! for forward-compatibility when the game adds new values.
//!
//! 将 RPC 和字段中使用的原始整数 ID 映射为可读的 Rust 枚举变体。
//! 插件作者可以直接 pattern match；事件上的原始 `i32` 字段保留，
//! 以便游戏新增值时向前兼容。

use std::fmt;

// ── DamageType ────────────────────────────────────────────────────────────

/// How the damage was dealt.
///
/// 伤害的施加方式。
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DamageType {
    Accident = 0,
    Bullet = 1,
    Launcher = 2,
    Grenade = 3,
    Shell = 4,
    VehicleExplosion = 5,
    Artillery = 6,
    Nuke = 7,
}

impl DamageType {
    /// Convert a raw integer to the enum. Returns `None` for unknown values.
    ///
    /// 将原始整数转为枚举。未知值返回 `None`。
    pub fn from_raw(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::Accident),
            1 => Some(Self::Bullet),
            2 => Some(Self::Launcher),
            3 => Some(Self::Grenade),
            4 => Some(Self::Shell),
            5 => Some(Self::VehicleExplosion),
            6 => Some(Self::Artillery),
            7 => Some(Self::Nuke),
            _ => None,
        }
    }

    /// Human-readable name.
    ///
    /// 人类可读名称。
    pub fn name(&self) -> &'static str {
        match self {
            Self::Accident => "accident",
            Self::Bullet => "bullet",
            Self::Launcher => "launcher",
            Self::Grenade => "grenade",
            Self::Shell => "shell",
            Self::VehicleExplosion => "vehicleExplosion",
            Self::Artillery => "artillery",
            Self::Nuke => "nuke",
        }
    }
}

impl fmt::Display for DamageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

// ── GadgetId ──────────────────────────────────────────────────────────────

/// Equipment / gadget slot items.
///
/// 装备 / 道具栏物品。
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GadgetId {
    Bazooka = 0,
    BandagePouch = 1,
    AmmoPouch = 2,
    Panzerschreck = 3,
}

impl GadgetId {
    pub fn from_raw(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::Bazooka),
            1 => Some(Self::BandagePouch),
            2 => Some(Self::AmmoPouch),
            3 => Some(Self::Panzerschreck),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Bazooka => "Bazooka",
            Self::BandagePouch => "BandagePouch",
            Self::AmmoPouch => "AmmoPouch",
            Self::Panzerschreck => "Panzerschreck",
        }
    }
}

impl fmt::Display for GadgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

// ── WeaponId ──────────────────────────────────────────────────────────────

/// Primary / secondary weapon identifiers.
///
/// 主武器 / 副武器标识。
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaponId {
    M1915 = 0,
    M1Garand = 1,
    Mp40 = 2,
    Kar98k = 3,
    Stg44 = 4,
    Mg42 = 5,
    Sten = 6,
    Mg34 = 7,
    M2Browning = 8,
    Welrod = 9,
}

impl WeaponId {
    pub fn from_raw(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::M1915),
            1 => Some(Self::M1Garand),
            2 => Some(Self::Mp40),
            3 => Some(Self::Kar98k),
            4 => Some(Self::Stg44),
            5 => Some(Self::Mg42),
            6 => Some(Self::Sten),
            7 => Some(Self::Mg34),
            8 => Some(Self::M2Browning),
            9 => Some(Self::Welrod),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::M1915 => "M1915",
            Self::M1Garand => "M1 Garand",
            Self::Mp40 => "MP40",
            Self::Kar98k => "Kar98k",
            Self::Stg44 => "STG 44",
            Self::Mg42 => "MG42",
            Self::Sten => "Sten",
            Self::Mg34 => "MG34",
            Self::M2Browning => "M2Browning",
            Self::Welrod => "Welrod",
        }
    }
}

impl fmt::Display for WeaponId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

// ── MatchType ─────────────────────────────────────────────────────────────

/// What kind of match is being played.
///
/// 当前在玩哪种比赛类型。
///
/// Backs `GameManager.matchType`. Variant names match the C# enum
/// (camelCase) so `to_string()` round-trips with [`crate::Ctx::match_type`].
///
/// 对应 `GameManager.matchType`。变体名与 C# 枚举一致（驼峰），因此
/// `to_string()` 与 [`crate::Ctx::match_type`] 的字符串等价。
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum MatchType {
    teamMatch = 0,
    conquest = 1,
}

impl MatchType {
    /// Convert a raw integer to the enum. Returns `None` for unknown values.
    ///
    /// 将原始整数转为枚举。未知值返回 `None`。
    pub fn from_raw(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::teamMatch),
            1 => Some(Self::conquest),
            _ => None,
        }
    }

    /// Variant name as it appears in the C# enum source.
    ///
    /// C# 源码中的变体名。
    pub fn name(&self) -> &'static str {
        match self {
            Self::teamMatch => "teamMatch",
            Self::conquest => "conquest",
        }
    }
}

impl fmt::Display for MatchType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Vehicle type enum from `VehicleControl.vehicleType`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum VehicleType {
    None = 0,
    Jeep = 1,
    Tank = 2,
    Airplane = 3,
}

impl VehicleType {
    pub fn from_raw(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::None),
            1 => Some(Self::Jeep),
            2 => Some(Self::Tank),
            3 => Some(Self::Airplane),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Jeep => "jeep",
            Self::Tank => "tank",
            Self::Airplane => "airplane",
        }
    }
}

impl fmt::Display for VehicleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
