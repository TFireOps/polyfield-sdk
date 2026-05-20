# 插件开发指南

> English: [`PLUGIN_GUIDE.md`](PLUGIN_GUIDE.md)
> API 速查：[`API_REFERENCE.md`](API_REFERENCE.md)

---

## 1. 准备

- 系统：Linux x86_64（Windows 用 WSL2，代码放 `~/` 不要放 `/mnt/`）
- 工具链：stable Rust
- 框架：已构建好的 `libpolyfield.so`，游戏目录里放好 `polyfield.toml`

---

## 2. 第一个插件

```bash
cargo new --lib my-plugin
cd my-plugin
```

`Cargo.toml`：

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
polyfield = { git = "https://github.com/TFireOps/polyfield-sdk" }
```

`src/lib.rs`：

```rust
use polyfield::{declare_plugin, manifest, Ctx, Plugin, PluginManifest};
use polyfield::events::PlayerJoinEvent;

#[derive(Default)]
struct Hello;

impl Plugin for Hello {
    fn manifest() -> &'static PluginManifest {
        manifest!(
            name = "hello",
            version = "0.1.0",
            authors = "you",
            description = "hello world",
        )
    }

    fn on_load(&mut self, ctx: &Ctx) {
        ctx.log_info("hello online");
    }

    fn on_player_join(&mut self, evt: &mut PlayerJoinEvent, ctx: &Ctx) -> bool {
        ctx.log_info(&format!("welcome {}", evt.name));
        true
    }
}

declare_plugin!(Hello::default());
```

构建并安装：

```bash
cargo build --release
cp target/release/libmy_plugin.so /path/to/game/plugins/
```

重启游戏，`polyfield.log` 里看到 `[hello] hello online` 就成功了。

---

## 3. 事件一览

| 方法 | 类型 | 触发时机 |
|---|---|---|
| `on_load(ctx)` | 通知 | 插件加载完 |
| `on_game_start(evt, ctx)` | 通知 | 新一局开始 |
| `on_player_join(evt, ctx) -> bool` | 可拦截 | 玩家加入 |
| `on_damage(evt, ctx) -> bool` | 可拦截 | 造成伤害 |
| `on_chat(evt, ctx) -> bool` | 可拦截 | 发聊天 |
| `on_respawn(evt, ctx) -> bool` | 可拦截 | 玩家重生 |
| `on_grenade(evt, ctx) -> bool` | 可拦截 | 扔手雷 |
| `on_shoot(evt, ctx) -> bool` | 可拦截 | 开枪 |
| `on_vehicle_shoot(evt, ctx) -> bool` | 可拦截 | 载具开火 |
| `on_vehicle_repair(evt, ctx) -> bool` | 可拦截 | 载具修理完成 |
| `on_reload(evt, ctx)` | 通知 | 换弹 |
| `on_latency(evt, ctx)` | 通知 | 延迟采样 |
| `on_tick(evt, ctx)` | 通知 | 每 50ms |

**可拦截事件**：返回 `true` 放行，`false` 拦截。可以修改 `evt` 的字段再放行。

---

## 4. 玩家与载具

### Player

```rust
let p = evt.player(ctx);       // 从事件获取
let p = ctx.player(ref);       // 从 PlayerRef 获取
let p = ctx.host_player();     // 房主
let p = ctx.player_by_name("xx");
let p = ctx.player_by_id(3);   // 槽位号
```

常用方法：

```rust
p.name()           // String
p.player_id()      // u32 (槽位)
p.health()         // i32
p.position()       // [f32; 3]
p.velocity()       // [f32; 3]
p.is_dead()        // bool
p.is_host()        // bool
p.ip()             // String
p.kill()
p.kick_with_reason("Banned", "原因", 0.5)
p.set_health(100, 0)
p.show_error("Title", "Body")
```

### Vehicle

```rust
let v = evt.vehicle(ctx);      // 从 VehicleShootEvent 获取
let v = ctx.vehicle(ref);      // 从指针获取
for v in ctx.vehicles() { }    // 遍历所有载具
```

常用方法：

```rust
v.health()         // i32
v.vehicle_type()   // Option<VehicleType>
v.position()       // [f32; 3]
v.velocity()       // [f32; 3]
v.rotation()       // [f32; 3]
v.driver()         // Option<Player>
```

### 跨事件追踪

存 `PlayerRef`（`u64`），不要存 `Player`：

```rust
use std::collections::HashMap;
use polyfield::events::PlayerRef;

struct MyPlugin {
    data: HashMap<PlayerRef, i64>,
}
```

---

## 5. 示例

### 伤害校验

```rust
fn on_damage(&mut self, evt: &mut DamageEvent, ctx: &Ctx) -> bool {
    let attacker = evt.attacker(ctx);
    if attacker.is_host() { return true; }

    if let Some(DamageType::Bullet) = evt.damage_type_enum() {
        if evt.amount > 500 {
            attacker.kick_with_reason("Banned", "异常伤害", 0.5);
            return false;
        }
    }
    true
}
```

### 聊天审核

```rust
fn on_chat(&mut self, evt: &mut ChatEvent, ctx: &Ctx) -> bool {
    if evt.message.contains("badword") {
        evt.message = evt.message.replace("badword", "****");
    }
    if evt.message.to_lowercase().contains("hack") {
        evt.sender(ctx).kick_with_reason("Banned", "禁用词", 0.5);
        return false;
    }
    true
}
```

### 定时检测

```rust
fn on_tick(&mut self, evt: &TickEvent, ctx: &Ctx) {
    if evt.frame % 100 == 0 {  // 每 5 秒
        let players = ctx.players();
        ctx.log_info(&format!("online: {}", players.len()));
    }
}
```

### 主机发消息

```rust
use polyfield::color;

ctx.host_say(&format!("{} welcome!", color("red", "Server")));
```

### 跨插件数据共享

```rust
ctx.kv_set("stats:kills", "42");
if let Some(val) = ctx.kv_get("stats:kills") { /* ... */ }
```

---

## 6. 构建运行

```bash
cargo build --release
cp target/release/libmy_plugin.so /path/to/game/plugins/
RUST_LOG=info LD_PRELOAD=/path/to/libpolyfield.so ./Polyfield.x86_64
```

---

## 7. 注意

- 不支持热重载，改完必须重启游戏
- 字段读取不会失败，无效时返回 `0` / `false` / `""` / `[0,0,0]`
- `Ctx` 不要存，离开回调就失效。跨回调用 `PlayerRef`
- 可拦截 hook 跑在游戏主线程，不要做重活
- `on_chat` 看不到房主自己说的话
- 多插件冲突：第一个 `false` 胜出
