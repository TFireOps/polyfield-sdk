//! `Vec3` — a tiny 3-component vector with the distance helpers anti-cheat
//! logic reaches for constantly.
//!
//! `Vec3` —— 一个三分量向量小类型，带反作弊逻辑常用的距离辅助方法。
//!
//! Position / velocity accessors on [`Player`](crate::Player) and
//! [`Vehicle`](crate::Vehicle) return raw `[f32; 3]` arrays so they stay
//! ABI-trivial. Wrap one in [`Vec3`] when you want distances and planar
//! math without hand-rolling `sqrt`:
//!
//! [`Player`](crate::Player) / [`Vehicle`](crate::Vehicle) 上的位置 / 速度
//! getter 返回原始 `[f32; 3]` 数组以保持 ABI 简单。需要距离或水平面运算
//! 时用 [`Vec3`] 包一层，免得自己手写 `sqrt`：
//!
//! ```
//! use polyfield::Vec3;
//! let a = Vec3::new(0.0, 0.0, 0.0);
//! let b = Vec3::from([3.0, 0.0, 4.0]);
//! assert_eq!(a.distance(b), 5.0);
//! ```

/// A 3-component `f32` vector. `Copy`, cheap, and convertible to/from the
/// `[f32; 3]` arrays the handle accessors return.
///
/// 三分量 `f32` 向量。`Copy`、轻量，可与句柄 getter 返回的 `[f32; 3]`
/// 互相转换。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    /// Construct from components.
    ///
    /// 用分量构造。
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// 3D Euclidean distance to `other`.
    ///
    /// 到 `other` 的三维欧氏距离。
    pub fn distance(self, other: Vec3) -> f32 {
        self.distance_sq(other).sqrt()
    }

    /// Squared 3D distance — skips the `sqrt`. Prefer this when only
    /// comparing against a threshold (`a.distance_sq(b) > r * r`).
    ///
    /// 三维距离的平方——省掉 `sqrt`。仅与阈值比较时优先用它
    /// （`a.distance_sq(b) > r * r`）。
    pub fn distance_sq(self, other: Vec3) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    /// Horizontal (XZ-plane) distance to `other`, ignoring the Y axis.
    /// This is the right metric for ground movement / speed-hack checks,
    /// where vertical motion (falling, stairs) shouldn't count.
    ///
    /// 到 `other` 的水平（XZ 平面）距离，忽略 Y 轴。这是地面移动 /
    /// 加速作弊检测该用的度量——垂直运动（下落、台阶）不应计入。
    pub fn distance_2d(self, other: Vec3) -> f32 {
        let dx = self.x - other.x;
        let dz = self.z - other.z;
        (dx * dx + dz * dz).sqrt()
    }

    /// Vector length (distance from origin).
    ///
    /// 向量长度（到原点的距离）。
    pub fn magnitude(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Horizontal speed: magnitude of the XZ components only. Apply to a
    /// velocity vector to get ground speed, ignoring vertical motion.
    ///
    /// 水平速度：仅 XZ 分量的模长。作用于速度向量即得地面速度，忽略
    /// 垂直运动。
    pub fn magnitude_2d(self) -> f32 {
        (self.x * self.x + self.z * self.z).sqrt()
    }

    /// The underlying `[x, y, z]` array.
    ///
    /// 底层的 `[x, y, z]` 数组。
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from(v: [f32; 3]) -> Self {
        Self {
            x: v[0],
            y: v[1],
            z: v[2],
        }
    }
}

impl From<Vec3> for [f32; 3] {
    fn from(v: Vec3) -> Self {
        [v.x, v.y, v.z]
    }
}
