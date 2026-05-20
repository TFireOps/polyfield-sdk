# 插件开发指南

> English: [`PLUGIN_GUIDE.md`](PLUGIN_GUIDE.md)
> 字段方法速查：[`API_REFERENCE.md`](API_REFERENCE.md)

写一个插件的完整流程。看完直接能写。

---

## 1. 准备

- 系统：Linux x86_64（Windows 用 WSL2，代码放 `~/` 不要放 `/mnt/`）
- 工具链：stable Rust
- 框架：已经构建好的 `libpolyfield.so`，游戏目录里放好 `polyfield.toml`

---

## 2. 第一个插件

新建 crate：

```bash
cargo new --lib polyfield-hello
cd polyfield-hello
```

`Cargo.toml`：

```toml
[package]
name = "polyfield-hello"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]   # 必须是 cdylib

[dependencies]
polyfield = { path = "../Polyfield_AntiCheat/crates/sdk" }
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
        true   // true = 放行；false = 拦截
    }
}

declare_plugin!(Hello::default());
```

构建并安装：

```bash
cargo build --release
cp target/release/libpolyfield_hello.so /path/to/Polyfield/plugins/
```

重启游戏，`polyfield.log` 里能看到 `[hello] hello online` 就成功了。

---

## 3. 事件一览

实现 `Plugin` trait 的对应方法即可。每个方法都有空的默认实现，只写你需要的。

| 方法 | 类型 | 触发时机 |
|---|---|---|
| `on_load(ctx)` | 通知 | 插件加载完 |
| `on_game_start(evt, ctx)` | 通知 | 新一局开始 |
| `on_player_join(evt, ctx) -> bool` | **可拦截** | 玩家加入 |
| `on_damage(evt, ctx) -> bool` | **可拦截** | 玩家造成伤害 |
| `on_chat(evt, ctx) -> bool` | **可拦截** | 玩家发聊天 |
| `on_latency(evt, ctx)` | 通知 | 延迟采样 |
| `on_tick(evt, ctx)` | 通知 | 每 50ms |

**可拦截事件返回值：** `true` 放行，`false` 拦截掉原始游戏调用。多个插件按加载顺序依次调用，第一个返回 `false` 的短路掉后面所有插件。

**事件参数 `&mut`：** 可拦截事件可以改字段（比如改 `evt.amount` 把伤害打折，改 `evt.message` 替换聊天内容），改完返回 `true` 让游戏用新值继续。

---

## 4. 玩家对象

三种类型，按场景选：

| 类型 | 何时用 |
|---|---|
| `Player<'ctx>` | 当前回调里读字段 / 调动作 |
| `PlayerRef`（`u64`） | 跨事件追踪，存进 `HashMap` 当 key |
| `PlayerInfo` | 把当前快照存起来 |

获取方式：

```rust
let p = evt.attacker(ctx);          // 从事件拿
let p = ctx.player(player_ref);     // 从 PlayerRef 拿
let p = ctx.host_player();          // 房主
let p = ctx.player_by_name("xx");   // 按名字找
```

常用读字段（完整列表见 [`API_REFERENCE.md`](API_REFERENCE.md)）：

```rust
p.name()         // String，显示名
p.player_id()    // u32，槽位 id（"Player3" -> 3）
p.health()       // i32
p.position()     // [f32; 3]
p.speed()        // f32
p.is_dead()      // bool
p.is_host()      // bool，是不是房主
p.weapon_id()    // i32
p.ip()           // String
```

常用动作：

```rust
p.kick_with_reason("Banned", "原因", 0.5);   // 显示弹窗后踢（推荐）
p.kill();
p.set_health(0, 0);
p.show_error("Title", "Body");
```

---

## 5. 示例

### 5.1 伤害校验

```rust
use polyfield::events::DamageEvent;
use polyfield::game_enums::DamageType;

fn on_damage(&mut self, evt: &mut DamageEvent, ctx: &Ctx) -> bool {
    let attacker = evt.attacker(ctx);

    if attacker.is_host() {
        return true;        // 不要踢房主
    }

    if let Some(DamageType::Bullet) = evt.damage_type_enum() {
        if evt.amount > 500 {
            attacker.kick_with_reason("Banned", "异常伤害", 0.5);
            return false;   // 拦掉这次伤害
        }
    }

    true
}
```

### 5.2 聊天审核

```rust
fn on_chat(&mut self, evt: &mut ChatEvent, ctx: &Ctx) -> bool {
    let sender = evt.sender(ctx);

    // 命令：/info
    if evt.message.trim() == "/info" {
        ctx.log_info(&format!("{} hp={}", sender.name(), sender.health()));
        return false;       // 吞掉这条消息
    }

    // 改写内容
    if evt.message.contains("badword") {
        evt.message = evt.message.replace("badword", "****");
    }

    // 拦截 + 踢
    if evt.message.to_lowercase().contains("hack") {
        sender.kick_with_reason("Banned", "禁用词", 0.5);
        return false;
    }

    true
}
```

### 5.3 累计统计（跨事件）

存 `PlayerRef`，不要存 `Player`。

```rust
use std::collections::HashMap;
use polyfield::events::{DamageEvent, PlayerRef};

#[derive(Default)]
struct Budget {
    total: HashMap<PlayerRef, i64>,
}

impl Plugin for Budget {
    // ... manifest ...

    fn on_damage(&mut self, evt: &mut DamageEvent, ctx: &Ctx) -> bool {
        let t = self.total.entry(evt.attacker).or_insert(0);
        *t += evt.amount as i64;
        if *t > 100_000 {
            evt.attacker(ctx).kick_with_reason("Banned", "总伤害超限", 0.5);
            return false;
        }
        true
    }

    fn on_game_start(&mut self, _evt: &GameStartEvent, _ctx: &Ctx) {
        self.total.clear();   // 新一局清零
    }
}
```

### 5.4 定时检测（移动 / 速度）

没有 `on_move`，用 `on_tick` 自己 diff：

```rust
use std::collections::HashMap;
use polyfield::events::{TickEvent, PlayerRef};

#[derive(Default)]
struct Anti {
    last_pos: HashMap<PlayerRef, [f32; 3]>,
}

const TICK_DT: f32 = 0.05;

impl Plugin for Anti {
    // ... manifest ...

    fn on_tick(&mut self, evt: &TickEvent, ctx: &Ctx) {
        for snap in ctx.players() {
            let p = ctx.player(snap.id);
            if p.is_dead() { continue; }

            // 速度异常
            if p.speed() > 20.0 && p.is_grounded() {
                ctx.log_warn(&format!("speed: {} {:.1}", p.name(), p.speed()));
            }

            // 瞬移检测
            let pos = p.position();
            if let Some(prev) = self.last_pos.get(&snap.id).copied() {
                let dx = pos[0] - prev[0];
                let dy = pos[1] - prev[1];
                let dz = pos[2] - prev[2];
                let dist = (dx*dx + dy*dy + dz*dz).sqrt();
                if dist / TICK_DT > 50.0 {
                    p.kick_with_reason("Kicked", "瞬移", 0.5);
                    continue;
                }
            }
            self.last_pos.insert(snap.id, pos);
        }

        // 每秒一次的便宜检查
        if evt.frame % 20 == 0 {
            // ...
        }
    }
}
```

---

## 6. 构建运行

```bash
# 构建插件
cargo build --release
# 产物：target/release/lib<crate-name>.so

# 复制到游戏的 plugins/ 目录
cp target/release/libpolyfield_hello.so /path/to/Polyfield/plugins/

# 运行游戏（每次改插件都要重启游戏）
RUST_LOG=info \
LD_PRELOAD=/path/to/libpolyfield.so \
./Polyfield.x86_64
```

成功加载会看到：

```
[polyfield] loading plugin: .../libpolyfield_hello.so
[polyfield] registered: hello v0.1.0
[hello] hello online
```

报 `ABI version mismatch` 就说明 SDK 和框架版本不匹配，重新编译。

---

## 7. 注意

- **不支持热重载。** 改完插件必须重启游戏。
- **字段读取不会失败。** 失败时返回 `0` / `false` / `""` / `[0,0,0]`，不是 `Option`。处理可疑值前自己加判断。
- **`Ctx` 不要存。** 它没有 `Send`/`Sync`，离开当前回调就失效。要跨回调用东西就存 `PlayerRef`。
- **可拦截 hook 不要做重活。** 它跑在游戏线程上。重活挪到 `on_tick`。
- **`on_chat` 看不到房主自己说的话。** 只 hook 客户端 → 房主的 RPC。
- **多插件冲突：第一个 `false` 胜出。** 后续插件不会被调用。

---

## 更多

- [`API_REFERENCE.md`](API_REFERENCE.md) — 全部字段、动作、枚举
- [`../crates/plugin-example/`](../crates/plugin-example/) — 完整示例插件
