//! `Vehicle<'ctx>` — zero-cost handle for reading VehicleControl fields
//! and performing vehicle-related queries. Mirrors the `Player<'ctx>`
//! pattern: cheap to construct, reads fields on demand, lifetime-bound
//! to the current callback.

use crate::abi::HostApi;
use crate::events::PlayerRef;
use crate::game_enums::VehicleType;
use crate::player::{read_string_via, Player};

/// A handle to a live `VehicleControl` instance. Constructed via
/// `ctx.vehicle(ref)` or `evt.vehicle(ctx)`. Cannot outlive the
/// callback it was created in.
pub struct Vehicle<'ctx> {
    id: PlayerRef,
    host: &'ctx HostApi,
}

impl<'ctx> Vehicle<'ctx> {
    pub(crate) fn new(id: PlayerRef, host: &'ctx HostApi) -> Self {
        Self { id, host }
    }

    /// Raw VehicleControl pointer as u64.
    pub fn raw(&self) -> PlayerRef {
        self.id
    }

    /// Vehicle health.
    pub fn health(&self) -> i32 {
        unsafe { (self.host.vehicle_health)(self.id) }
    }

    /// Vehicle type as raw i32.
    pub fn vehicle_type_raw(&self) -> i32 {
        unsafe { (self.host.vehicle_type)(self.id) }
    }

    /// Vehicle type as typed enum.
    pub fn vehicle_type(&self) -> Option<VehicleType> {
        VehicleType::from_raw(self.vehicle_type_raw())
    }

    /// `VehicleControl.vehicleModel` name (e.g. `"jagdpanther"`). Empty
    /// string if the field can't be resolved. Distinct from
    /// [`vehicle_type`](Self::vehicle_type), which is the coarse category;
    /// this is the specific model string the game uses.
    ///
    /// `VehicleControl.vehicleModel` 的名字（如 `"jagdpanther"`）。字段
    /// 无法解析时为空串。与粗分类的 [`vehicle_type`](Self::vehicle_type)
    /// 不同，这是游戏使用的具体模型字符串。
    pub fn model_name(&self) -> String {
        read_string_via(|buf, cap| unsafe {
            (self.host.vehicle_model_name)(self.id, buf, cap)
        })
    }

    /// The driver's PlayerRef. Returns 0 if no driver.
    pub fn driver_ref(&self) -> PlayerRef {
        unsafe { (self.host.vehicle_driver)(self.id) }
    }

    /// The driver as a `Player` handle. `None` if no driver.
    pub fn driver(&self) -> Option<Player<'ctx>> {
        let id = self.driver_ref();
        (id != 0).then(|| Player::new(id, self.host))
    }

    /// Vehicle world position `[x, y, z]`.
    pub fn position(&self) -> [f32; 3] {
        let mut v = [0f32; 3];
        unsafe { (self.host.vehicle_position)(self.id, v.as_mut_ptr()) };
        v
    }

    /// Vehicle rotation (euler angles) `[x, y, z]`.
    pub fn rotation(&self) -> [f32; 3] {
        let mut v = [0f32; 3];
        unsafe { (self.host.vehicle_rotation)(self.id, v.as_mut_ptr()) };
        v
    }

    /// Vehicle velocity `[x, y, z]`.
    pub fn velocity(&self) -> [f32; 3] {
        let mut v = [0f32; 3];
        unsafe { (self.host.vehicle_velocity)(self.id, v.as_mut_ptr()) };
        v
    }
}
