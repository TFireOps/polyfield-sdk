//! Reference plugin for Polyfield.
//!
//! 不做任何反作弊判断，仅把每个事件的来源（玩家）和入参打印出来，
//! 方便插件作者直观看到 SDK 暴露了哪些信息。
//!
//! Build:
//!   cargo build --release -p plugin-example
//! Output:
//!   target/release/libpolyfield_example.so
//!
//! Drop that file into the framework's `plugins/` directory.

use polyfield::events::{
    ChatEvent, DamageEvent, GameStartEvent, GrenadeEvent, LatencySample, PlayerJoinEvent,
    ReloadEvent, RespawnEvent, ShootEvent, TickEvent, VehicleRepairEvent, VehicleShootEvent,
};
use polyfield::{declare_plugin, manifest, Ctx, Plugin, PluginManifest};

pub struct Example;

impl Example {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for Example {
    fn manifest() -> &'static PluginManifest {
        manifest!(
            name = "example",
            version = "0.1.0",
            authors = "polyfield-team",
            description = "event tracer: logs caller + params for every event",
        )
    }

    fn on_load(&mut self, ctx: &Ctx) {
        ctx.log_info("example plugin online");
    }

    fn on_game_start(&mut self, evt: &GameStartEvent, ctx: &Ctx) {
        ctx.log_info(
            format!(
                "[on_game_start] frame={} map={:?} match_type={:?}",
                evt.frame,
                ctx.game_map(),
                ctx.match_type(),
            )
            .as_str(),
        );
    }

    fn on_player_join(&mut self, evt: &PlayerJoinEvent, ctx: &Ctx) {
        let p = evt.player(ctx);
        ctx.log_info(
            format!(
                "[on_player_join] caller={}({}) ip={:?} name_param={:?}",
                p.name(),
                p.player_id(),
                p.ip(),
                evt.name,
            )
            .as_str(),
        );
    }

    fn on_damage(&mut self, evt: &mut DamageEvent, ctx: &Ctx) -> bool {
        let attacker = evt.attacker(ctx);
        let victim = evt
            .victim(ctx)
            .map(|v| format!("{}({})", v.name(), v.player_id()))
            .unwrap_or_else(|| "<npc>".to_string());
        ctx.log_info(
            format!(
                "[on_damage] caller={}({}) victim={} amount={} dmg_type={:?} weapon={:?} is_npc={} data={:?}",
                attacker.name(),
                attacker.player_id(),
                victim,
                evt.amount,
                evt.damage_type_enum(),
                evt.weapon_enum(),
                evt.is_npc,
                evt.data,
            )
            .as_str(),
        );
        true
    }

    fn on_chat(&mut self, evt: &mut ChatEvent, ctx: &Ctx) -> bool {
        let sender = evt.sender(ctx);
        ctx.log_info(
            format!(
                "[on_chat] caller={}({}) message={:?}",
                sender.name(),
                sender.player_id(),
                evt.message,
            )
            .as_str(),
        );
        true
    }

    fn on_respawn(&mut self, evt: &mut RespawnEvent, ctx: &Ctx) -> bool {
        let p = evt.player(ctx);
        ctx.log_info(
            format!(
                "[on_respawn] caller={}({}) spawn_data={:?} vehicle_type={}",
                p.name(),
                p.player_id(),
                evt.spawn_data,
                evt.vehicle_type,
            )
            .as_str(),
        );
        true
    }

    fn on_grenade(&mut self, evt: &mut GrenadeEvent, ctx: &Ctx) -> bool {
        if evt.player == 0 {
            ctx.log_info(
                format!(
                    "[on_grenade] caller=<unresolved> grenade_data={:?}",
                    evt.grenade_data,
                )
                .as_str(),
            );
            return true;
        }
        let p = evt.player(ctx);
        ctx.log_info(
            format!(
                "[on_grenade] caller={}({}) grenade_data={:?}",
                p.name(),
                p.player_id(),
                evt.grenade_data,
            )
            .as_str(),
        );
        true
    }

    fn on_shoot(&mut self, evt: &mut ShootEvent, ctx: &Ctx) -> bool {
        if evt.player == 0 {
            ctx.log_info(
                format!(
                    "[on_shoot] caller=<unresolved> weapon_type={} weapon={:?} shoot_data={:?}",
                    evt.weapon_type,
                    evt.weapon_enum(),
                    evt.shoot_data,
                )
                .as_str(),
            );
            return true;
        }
        let p = evt.player(ctx);
        ctx.log_info(
            format!(
                "[on_shoot] caller={}({}) weapon_type={} weapon={:?} shoot_data={:?}",
                p.name(),
                p.player_id(),
                evt.weapon_type,
                evt.weapon_enum(),
                evt.shoot_data,
            )
            .as_str(),
        );
        true
    }

    fn on_latency(&mut self, evt: &LatencySample, ctx: &Ctx) {
        let p = evt.player(ctx);
        ctx.log_info(
            format!(
                "[on_latency] caller={}({}) ms={:.0}",
                p.name(),
                p.player_id(),
                evt.ms,
            )
            .as_str(),
        );
    }

    fn on_reload(&mut self, evt: &ReloadEvent, ctx: &Ctx) {
        let p = evt.player(ctx);
        ctx.log_info(
            format!(
                "[on_reload] caller={}({}) anim_name={:?}",
                p.name(),
                p.player_id(),
                evt.anim_name,
            )
            .as_str(),
        );
    }

    fn on_vehicle_shoot(&mut self, evt: &mut VehicleShootEvent, ctx: &Ctx) -> bool {
        let p = evt.player(ctx);
        let v = evt.vehicle(ctx);
        ctx.log_info(
            format!(
                "[on_vehicle_shoot] caller={}({}) vehicle_id={} seat={} veh_health={} veh_type={:?}",
                p.name(),
                p.player_id(),
                evt.vehicle_id,
                evt.seat_id,
                v.health(),
                v.vehicle_type(),
            )
            .as_str(),
        );
        true
    }

    fn on_vehicle_repair(&mut self, evt: &mut VehicleRepairEvent, ctx: &Ctx) -> bool {
        let p = evt.player(ctx);
        ctx.log_info(
            format!(
                "[on_vehicle_repair] caller={}({}) vehicle_id={} timer={} health={}",
                p.name(),
                p.player_id(),
                evt.vehicle_id,
                evt.timer,
                evt.health,
            )
            .as_str(),
        );
        true
    }

    fn on_tick(&mut self, evt: &TickEvent, ctx: &Ctx) {
        // 5s = 100 ticks (50ms each)
        if evt.frame % 100 == 0 {
            let players = ctx.players();
            let names: Vec<String> = players
                .iter()
                .map(|s| ctx.player(s.id).name())
                .collect();
            ctx.log_info(
                format!(
                    "[players] count={} names={:?}",
                    players.len(),
                    names,
                )
                .as_str(),
            );
        }
    }
}

declare_plugin!(Example::new());
