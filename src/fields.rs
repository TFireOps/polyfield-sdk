//! Field identifiers for `PlayerControl`.
//!
//! `PlayerControl` 字段标识。
//!
//! Both the framework and plugins share these `u32` IDs so the host
//! vtable only needs six generic reader slots (one per primitive
//! shape) regardless of how many fields exist.
//!
//! 框架和插件共享这些 `u32` ID，宿主 vtable 因此只需要六个通用 reader
//! slot（按基本类型分），与字段数量无关。
//!
//! When you need a new field:
//!   1. Add a constant here.
//!   2. Map it to its offset & type in `core/src/player_api.rs::OFFSETS`.
//!   3. Expose a typed accessor on `Player` in `sdk/src/player.rs`.
//! ABI does not need to bump.
//!
//! 新增字段流程：
//!   1. 这里加一个常量；
//!   2. 在 `core/src/player_api.rs::OFFSETS` 里登记偏移和类型；
//!   3. 在 `sdk/src/player.rs::Player` 上写一个带类型的 getter 方法。
//! ABI 版本无需 bump。

#![allow(non_upper_case_globals)]

/// `PlayerControl` field identifier. The `u32` value is opaque — only
/// the framework and SDK need to agree on it.
pub type PlayerField = u32;

// Identity / network
pub const F_PLAYER_ID: PlayerField = 0x0001;     // string  _playerID
pub const F_DEVICE_ID: PlayerField = 0x0002;     // string  deviceID
pub const F_TEAM: PlayerField = 0x0003;          // string  team
pub const F_GROUND_TYPE: PlayerField = 0x0004;   // string  groundType (private)

// Lifecycle / state
pub const F_HEALTH: PlayerField = 0x0100;        // int     health
pub const F_HEALTH_REGEN_COOLDOWN: PlayerField = 0x0101; // float
pub const F_DEAD: PlayerField = 0x0102;          // bool
pub const F_READY: PlayerField = 0x0103;         // bool
pub const F_RESPAWN_TIMER: PlayerField = 0x0104; // float
pub const F_DONE_LOADING_MAP: PlayerField = 0x0105; // bool
pub const F_USER_STATE: PlayerField = 0x0106;    // UserState (i32 enum)
pub const F_CLASS_ROLE: PlayerField = 0x0107;    // ClassRole (i32 enum)

// Stats / counters
pub const F_KILL_COUNT: PlayerField = 0x0200;       // int
pub const F_DEATH_COUNT: PlayerField = 0x0201;      // int
pub const F_BULLETS_FIRED: PlayerField = 0x0202;    // int
pub const F_GRENADES_THROWN: PlayerField = 0x0203;  // int
pub const F_RELOADS_DONE: PlayerField = 0x0204;     // int
pub const F_KILL_RATE: PlayerField = 0x0205;        // int
pub const F_DAMAGE_RATE: PlayerField = 0x0206;      // int
pub const F_NETWORK_RATE: PlayerField = 0x0207;     // int
pub const F_LATENCY_RATE: PlayerField = 0x0208;     // int
pub const F_PING_WARN: PlayerField = 0x0209;        // int
pub const F_TEAMKILL_WARN: PlayerField = 0x020A;    // int

// Movement / pose
pub const F_PLAYER_SPEED: PlayerField = 0x0300;     // float
pub const F_RUNNING: PlayerField = 0x0301;          // bool
pub const F_GROUNDED: PlayerField = 0x0302;         // bool
pub const F_CROUCH: PlayerField = 0x0303;           // int
pub const F_IS_UNDER_WATER: PlayerField = 0x0304;   // bool
pub const F_LAST_PLAYER_POS: PlayerField = 0x0305;  // Vec3 (stale, prefer F_NET_POSITION)
pub const F_MY_RIGID_VEL: PlayerField = 0x0306;     // Vec3
pub const F_MOVE_DIR: PlayerField = 0x0307;         // Vec3
pub const F_LOOK_DIR: PlayerField = 0x0308;         // Vec2

// PlayerNetTransform fields (accessed via _netTransform pointer)
pub const F_NET_POSITION: PlayerField = 0x0310;     // Vec3  _netTransform._recivedPos
pub const F_NET_VELOCITY: PlayerField = 0x0311;     // Vec3  _netTransform._recivedVel

// PlayerCombat fields (accessed via playerCombat pointer)
pub const F_WEAPON_ID: PlayerField = 0x0320;        // int   playerCombat.currWeaponID

// Combat-adjacent
pub const F_TRYING_TO_ATTACK: PlayerField = 0x0400; // float
pub const F_OBSTACLE_TIMER: PlayerField = 0x0401;   // float
pub const F_EXPOSE_TIMER: PlayerField = 0x0402;     // float
pub const F_DONT_EXPOSE: PlayerField = 0x0403;      // bool

// Input / camera
pub const F_MOUSE_X: PlayerField = 0x0500;          // float
pub const F_MOUSE_Y: PlayerField = 0x0501;          // float
pub const F_INPUT_X: PlayerField = 0x0502;          // float
pub const F_INPUT_Y: PlayerField = 0x0503;          // float
pub const F_AUTO_SPRINT: PlayerField = 0x0504;      // bool
pub const F_HEAD_BOB: PlayerField = 0x0505;         // bool
pub const F_JOYSTICK_LEAN: PlayerField = 0x0506;    // bool
pub const F_CAM_SENSITIVITY: PlayerField = 0x0507;  // float
pub const F_ADS_SENSITIVITY: PlayerField = 0x0508;  // float
pub const F_GYRO_LOOK_SENSITIVITY: PlayerField = 0x0509;  // float
pub const F_GYRO_ADS_SENSITIVITY: PlayerField = 0x050A;   // float
pub const F_LOCAL_CAM_DIST: PlayerField = 0x050B;   // float
pub const F_CAM_FOV: PlayerField = 0x050C;          // float
pub const F_CAM_SHAKE: PlayerField = 0x050D;        // float
pub const F_DEFAULT_LOD_BIAS: PlayerField = 0x050E; // float

// Network / latency
pub const F_MY_LATENCY: PlayerField = 0x0600;       // float

// Voting
pub const F_VOTE_KICKED: PlayerField = 0x0700;      // bool
pub const F_VOTED: PlayerField = 0x0701;            // bool
