use rand::Rng;
use rand::seq::SliceRandom;
use serde_json::Value;

use crate::combatsimulator::{
    ability::Ability,
    buff::Buff,
    combat_unit::CombatUnit,
    combat_utilities::CombatUtilities,
    event_queue::EventQueue,
    events::{CombatEvent, EventKind, UnitIdx},
    labyrinth::Labyrinth,
    monster::{MonsterBuilder, monster_update_combat_details},
    player::PlayerExt,
    sim_result::SimResult,
    zone::Zone,
};

const ONE_SECOND: i64 = 1_000_000_000;
const HOT_TICK_INTERVAL: i64 = 5 * ONE_SECOND;
const DOT_TICK_INTERVAL: i64 = 3 * ONE_SECOND;
const REGEN_TICK_INTERVAL: i64 = 10 * ONE_SECOND;
const ENEMY_RESPAWN_INTERVAL: i64 = 3 * ONE_SECOND;
const PLAYER_RESPAWN_INTERVAL: i64 = 150 * ONE_SECOND;
const RESTART_INTERVAL: i64 = 3 * ONE_SECOND;
const ENRAGE_TICK_INTERVAL: i64 = 60 * ONE_SECOND;

/// Split borrow helper: get mutable refs to two different elements of a Vec.
fn borrow_two_mut<T>(v: &mut Vec<T>, a: usize, b: usize) -> (&mut T, &mut T) {
    assert_ne!(a, b);
    if a < b {
        let (left, right) = v.split_at_mut(b);
        (&mut left[a], &mut right[0])
    } else {
        let (left, right) = v.split_at_mut(a);
        (&mut right[0], &mut left[b])
    }
}

fn get_skill_level(unit: &CombatUnit, skill: &str) -> f64 {
    match skill {
        "stamina" => unit.combat_details.stamina_level,
        "intelligence" => unit.combat_details.intelligence_level,
        "attack" => unit.combat_details.attack_level,
        "melee" => unit.combat_details.melee_level,
        "defense" => unit.combat_details.defense_level,
        "ranged" => unit.combat_details.ranged_level,
        "magic" => unit.combat_details.magic_level,
        _ => 1.0,
    }
}

pub struct CombatSimulator {
    units: Vec<CombatUnit>,
    num_players: usize,
    enemy_indices: Option<Vec<UnitIdx>>,
    zone: Option<Zone>,
    labyrinth: Option<Labyrinth>,
    event_queue: EventQueue,
    sim_result: SimResult,
    simulation_time: i64,
    all_players_dead: bool,
    enable_hp_mp_visualization: bool,
    enrage_begin_time: i64,
    temp_dungeon_count: i32,
    wipe_log_buffer: Vec<Value>,
    wipe_log_index: usize,
    wipe_log_count: usize,
    wipe_log_max: usize,
    market_prices: Option<std::collections::HashMap<String, f64>>,
    stop_after_dungeon_result: bool,
    guild_trial_mode: bool,
}

impl CombatSimulator {
    pub fn new(
        players: Vec<CombatUnit>,
        zone: Option<Zone>,
        labyrinth: Option<Labyrinth>,
        enable_hp_mp_visualization: bool,
        market_prices: Option<std::collections::HashMap<String, f64>>,
    ) -> Self {
        let num_players = players.len();
        let is_labyrinth = labyrinth.is_some();

        let sim_result = SimResult::new(
            zone.as_ref().map(|z| z.hrid.as_str()),
            zone.as_ref().map(|z| z.difficulty_tier),
            labyrinth.as_ref().map(|l| l.monster_hrid.as_str()),
            labyrinth.as_ref().map(|l| l.room_level),
            num_players,
            is_labyrinth,
        );

        let wipe_log_max = 200;
        let mut units = players;
        units.reserve(32);

        CombatSimulator {
            units,
            num_players,
            enemy_indices: None,
            zone,
            labyrinth,
            event_queue: EventQueue::new(),
            sim_result,
            simulation_time: 0,
            all_players_dead: false,
            enable_hp_mp_visualization,
            enrage_begin_time: 0,
            temp_dungeon_count: 0,
            wipe_log_buffer: vec![Value::Null; wipe_log_max],
            wipe_log_index: 0,
            wipe_log_count: 0,
            wipe_log_max,
            market_prices,
            stop_after_dungeon_result: false,
            guild_trial_mode: false,
        }
    }

    /// When enabled, `simulate` stops as soon as the first dungeon attempt in
    /// the run resolves (cleared or wiped) instead of running to `time_limit`.
    /// Used for the `--guild` trial staircase, where each guild tier is a
    /// single attempt rather than a repeatedly-farmed dungeon.
    pub fn set_stop_after_dungeon_result(&mut self, stop: bool) {
        self.stop_after_dungeon_result = stop;
    }

    /// Guild-trial-only parry rule: each incoming attack can receive up to 5
    /// parry attempts, one per parry-capable unit (e.g. Regal Sword users) in
    /// the party, each rolling independently; the first success cancels the
    /// whole attack. Zone/labyrinth keep the legacy single-roll model.
    pub fn set_guild_trial_mode(&mut self, enabled: bool) {
        self.guild_trial_mode = enabled;
    }

    // -- Wipe logs -------------------------------------------------------------

    fn add_to_wipe_logs(&mut self, entry: Value) {
        self.wipe_log_buffer[self.wipe_log_index] = entry;
        self.wipe_log_index = (self.wipe_log_index + 1) % self.wipe_log_max;
        self.wipe_log_count = (self.wipe_log_count + 1).min(self.wipe_log_max);
    }

    fn get_ordered_wipe_logs(&self) -> Vec<Value> {
        (0..self.wipe_log_count).map(|i| {
            let idx = (self.wipe_log_index + self.wipe_log_max - self.wipe_log_count + i) % self.wipe_log_max;
            self.wipe_log_buffer[idx].clone()
        }).collect()
    }

    fn save_wipe_logs_to_sim_result(&mut self, wave: i32) {
        let logs = self.get_ordered_wipe_logs();
        self.sim_result.add_wipe_event(logs, self.simulation_time, wave);
        self.wipe_log_index = 0;
        self.wipe_log_count = 0;
    }

    fn build_combat_log(&self, source_hrid: &str, ability: &str, target_idx: UnitIdx, damage: i64) -> Value {
        let target = &self.units[target_idx];
        let after_hp = target.combat_details.current_hitpoints;
        let before_hp = (after_hp + damage).max(0);
        let players_hp: Vec<Value> = (0..self.num_players).map(|i| {
            let p = &self.units[i];
            serde_json::json!({ "hrid": p.hrid, "current": p.combat_details.current_hitpoints, "max": p.combat_details.max_hitpoints })
        }).collect();
        serde_json::json!({
            "time": self.simulation_time, "source": source_hrid, "ability": ability,
            "target": target.hrid, "damage": damage, "beforeHp": before_hp,
            "afterHp": after_hp, "playersHp": players_hp, "isCrit": false
        })
    }

    // -- Main loop -------------------------------------------------------------

    pub fn simulate(&mut self, time_limit: i64) -> &SimResult {
        self.reset_simulation();
        self.event_queue.add_event(CombatEvent { time: 0, kind: EventKind::CombatStart });

        let mut ticks = 0u64;
        while self.simulation_time < time_limit {
            let next = match self.event_queue.get_next_event() {
                Some(e) => e,
                None => break,
            };
            self.process_event(next);
            ticks += 1;
            if ticks % 1000 == 0 && self.enable_hp_mp_visualization {
                self.sim_result.add_time_series_snapshot(
                    self.simulation_time,
                    &self.units[..self.num_players].to_vec(),
                );
            }
            if self.stop_after_dungeon_result && self.sim_result.dungeon_attempt_won.is_some() {
                break;
            }
        }

        // If the attempt neither cleared nor wiped (e.g. the caller's time_limit
        // ran out mid-encounter), still record how much enemy HP was left so a
        // guild-staircase timeout can report "how close" the party got, same as
        // a wipe does below.
        if self.sim_result.dungeon_attempt_won.is_none() {
            if let Some(ei) = self.enemy_indices.clone() {
                let total_max: i64 = ei.iter().map(|&i| self.units[i].combat_details.max_hitpoints).sum();
                let total_cur: i64 = ei.iter()
                    .map(|&i| self.units[i].combat_details.current_hitpoints.max(0))
                    .sum();
                if total_max > 0 {
                    self.sim_result.enemy_hp_pct_remaining =
                        Some(total_cur as f64 / total_max as f64 * 100.0);
                }
            }
        }

        // Finalize
        self.sim_result.is_dungeon = self.zone.as_ref().map(|z| z.is_dungeon).unwrap_or(false);
        if let Some(ref zone) = self.zone {
            if zone.is_dungeon {
                self.sim_result.dungeons_completed = zone.dungeons_completed;
                self.sim_result.dungeons_failed = zone.dungeons_failed;
                let max_waves = zone.dungeon_spawn_info["maxWaves"].as_i64().unwrap_or(0) as i32;
                if self.sim_result.dungeons_completed < 1 {
                    self.sim_result.max_wave_reached = 0;
                    for i in 1..=max_waves {
                        let wave_name = format!("#{}", i);
                        match self.sim_result.time_spent_alive.iter().find(|e| e.name == wave_name) {
                            Some(e) if e.count > 0 => self.sim_result.max_wave_reached = i,
                            _ => break,
                        }
                    }
                } else {
                    self.sim_result.max_wave_reached = max_waves;
                }
            }
        }
        self.sim_result.simulated_time = self.simulation_time;

        for i in 0..self.num_players {
            let unit = self.units[i].clone();
            self.sim_result.set_drop_rate_multipliers(&unit);
            self.sim_result.set_mana_used(&unit);
        }
        self.sim_result.compute_loot_value(self.market_prices.as_ref());

        if let Some(ref zone) = self.zone {
            if zone.is_dungeon {
                if let Some(fixed_map) = zone.dungeon_spawn_info["fixedSpawnsMap"].as_object() {
                    for (wave, monsters) in fixed_map {
                        let mut wave_name = format!("#{}", wave);
                        if let Some(arr) = monsters.as_array() {
                            for m in arr {
                                if let Some(h) = m["combatMonsterHrid"].as_str() {
                                    wave_name = format!("{},{}", wave_name, h);
                                }
                            }
                        }
                        self.sim_result.boss_spawns.push(wave_name);
                    }
                }
                if let Some(boss_spawns) = zone.monster_spawn_info["bossSpawns"].as_array() {
                    for boss in boss_spawns {
                        if let Some(h) = boss["combatMonsterHrid"].as_str() {
                            self.sim_result.boss_spawns.push(h.to_string());
                        }
                    }
                }
            }
        }
        if let Some(ref labyrinth) = self.labyrinth {
            self.sim_result.laby_attempt_count = labyrinth.attempt_count;
        }

        &self.sim_result
    }

    fn reset_simulation(&mut self) {
        self.temp_dungeon_count = 0;
        self.simulation_time = 0;
        self.event_queue.clear();
        self.units.truncate(self.num_players);
        self.enemy_indices = None;

        let is_labyrinth = self.labyrinth.is_some();
        let zone_hrid = self.zone.as_ref().map(|z| z.hrid.clone());
        let zone_tier = self.zone.as_ref().map(|z| z.difficulty_tier);
        let laby_hrid = self.labyrinth.as_ref().map(|l| l.monster_hrid.clone());
        let room_level = self.labyrinth.as_ref().map(|l| l.room_level);

        self.sim_result = SimResult::new(
            zone_hrid.as_deref(),
            zone_tier,
            laby_hrid.as_deref(),
            room_level,
            self.num_players,
            is_labyrinth,
        );
    }

    // -- Event dispatch --------------------------------------------------------

    fn process_event(&mut self, event: CombatEvent) {
        self.simulation_time = event.time;
        match event.kind.clone() {
            EventKind::CombatStart                              => self.process_combat_start(event.time),
            EventKind::PlayerRespawn { hrid }                  => self.process_player_respawn(&hrid),
            EventKind::EnemyRespawn                            => self.start_new_encounter(),
            EventKind::AutoAttack { source }                   => self.process_auto_attack(source),
            EventKind::ConsumableTick { source, consumable_hrid, total_ticks, current_tick }
                => self.process_consumable_tick(source, &consumable_hrid, total_ticks, current_tick),
            EventKind::DamageOverTime { source_ref, target, damage, total_ticks, current_tick, combat_style_hrid }
                => self.process_dot_tick(source_ref, target, damage, total_ticks, current_tick, &combat_style_hrid),
            EventKind::CheckBuffExpiration { source } => {
                let t = self.simulation_time;
                self.units[source].remove_expired_buffs(t);
            },
            EventKind::RegenTick                               => self.process_regen_tick(),
            EventKind::StunExpiration { source } => {
                self.units[source].is_stunned = false;
                self.add_next_attack_event(source);
            },
            EventKind::BlindExpiration { source } => {
                self.units[source].is_blinded = false;
                self.add_next_attack_event(source);
            },
            EventKind::SilenceExpiration { source } => {
                self.units[source].is_silenced = false;
            },
            EventKind::CurseExpiration { source, .. } => {
                let t = self.simulation_time;
                self.units[source].remove_expired_buffs(t);
            },
            EventKind::WeakenExpiration { source, .. } => {
                let t = self.simulation_time;
                self.units[source].remove_expired_buffs(t);
            },
            EventKind::FuryExpiration { source, .. } => {
                let t = self.simulation_time;
                self.units[source].remove_expired_buffs(t);
            },
            EventKind::EnrageTick { encounter_time }           => self.process_enrage_tick(encounter_time),
            EventKind::AbilityCastEnd { source, ability_idx }  => { self.try_use_ability(source, ability_idx); },
            EventKind::AwaitCooldown { source }                => self.add_next_attack_event(source),
            EventKind::CooldownReady                           => {},
        }
        self.check_triggers();
    }

    // -- Combat start ----------------------------------------------------------

    fn process_combat_start(&mut self, event_time: i64) {
        for i in 0..self.num_players {
            if event_time == 0 {
                self.units[i].generate_permanent_buffs();
                self.units[i].player_update_combat_details();
            }
            let reset_time = if self.labyrinth.is_some() { 0 } else { self.simulation_time };
            self.units[i].reset(reset_time);
        }
        self.event_queue.add_event(CombatEvent {
            time: self.simulation_time + REGEN_TICK_INTERVAL,
            kind: EventKind::RegenTick,
        });
        self.start_new_encounter();
    }

    // -- Player respawn --------------------------------------------------------

    fn process_player_respawn(&mut self, hrid: &str) {
        let idx = (0..self.num_players).find(|&i| self.units[i].hrid == hrid);
        if let Some(i) = idx {
            let mhp = self.units[i].combat_details.max_hitpoints;
            let mmp = self.units[i].combat_details.max_manapoints;
            self.units[i].combat_details.current_hitpoints = mhp;
            self.units[i].combat_details.current_manapoints = mmp;
            self.units[i].clear_buffs();
            self.units[i].clear_ccs();
            if hrid == "player1" && std::env::var("MWI_DEBUG_AUTOATTACK").is_ok() {
                eprintln!("[{:.1}s] RESPAWN player1: all_players_dead={} enemy_indices={:?} mp={}/{}",
                    self.simulation_time as f64 / 1e9, self.all_players_dead,
                    self.enemy_indices, mmp, mmp);
            }
            if self.all_players_dead {
                self.all_players_dead = false;
                self.start_attacks();
            } else {
                self.add_next_attack_event(i);
            }
        }
    }

    // -- New encounter ---------------------------------------------------------

    fn start_new_encounter(&mut self) {
        if self.all_players_dead {
            self.all_players_dead = false;
            if let Some(ref mut zone) = self.zone {
                zone.fail_wave();
            }
        }

        // Clear any buff-expiration or CC-expiration events that reference enemy
        // unit indices from the previous encounter. These indices become invalid
        // after units.truncate(num_players) and would cause an out-of-bounds panic.
        let num_players = self.num_players;
        self.event_queue.clear_matching(|e| {
            let source = match &e.kind {
                EventKind::CurseExpiration     { source, .. } => Some(*source),
                EventKind::WeakenExpiration    { source, .. } => Some(*source),
                EventKind::FuryExpiration      { source, .. } => Some(*source),
                EventKind::CheckBuffExpiration { source }     => Some(*source),
                EventKind::StunExpiration      { source }     => Some(*source),
                EventKind::BlindExpiration     { source }     => Some(*source),
                EventKind::SilenceExpiration   { source }     => Some(*source),
                EventKind::AutoAttack          { source }     => Some(*source),
                EventKind::AbilityCastEnd      { source, .. } => Some(*source),
                EventKind::AwaitCooldown       { source }     => Some(*source),
                EventKind::ConsumableTick      { source, .. } => Some(*source),
                _ => None,
            };
            source.map_or(false, |s| s >= num_players)
        });

        self.units.truncate(self.num_players);

        let new_enemies: Vec<CombatUnit> = if let Some(ref mut zone) = self.zone {
            if !zone.is_dungeon {
                zone.get_random_encounter(num_players as i32)
            } else {
                let wave_idx = zone.encounters_killed;
                let wave = zone.get_next_wave(num_players as i32);
                let wave_name = format!("#{}", wave_idx);
                self.sim_result.update_time_spent_alive(&wave_name, true, self.simulation_time);
                let cur_count = zone.dungeons_completed;
                if cur_count > self.temp_dungeon_count {
                    self.temp_dungeon_count = cur_count;
                    for i in 0..self.num_players {
                        let mhp = self.units[i].combat_details.max_hitpoints;
                        let mmp = self.units[i].combat_details.max_manapoints;
                        self.units[i].combat_details.current_hitpoints = mhp;
                        self.units[i].combat_details.current_manapoints = mmp;
                        // A player who died with an active CC (e.g. silence from
                        // a monster ability) right before the dungeon's final wave
                        // completes would otherwise carry that CC flag forward
                        // into the freshly-reset dungeon attempt: the unit that
                        // would have cleared it (SilenceExpiration, etc.) was
                        // correctly cancelled on death via clear_events_for_unit,
                        // but nothing else ever clears the flag in a dungeon
                        // (process_player_respawn, which does clear_ccs(), is
                        // never scheduled while is_dungeon == true). Left stuck,
                        // is_silenced permanently blocks the ability loop, so
                        // every cycle silently falls through to a real auto-attack.
                        self.units[i].clear_ccs();
                    }
                }
                wave
            }
        } else if let Some(ref mut laby) = self.labyrinth {
            let m = laby.get_monster();
            let t = self.simulation_time;
            laby.update_encounter_start_time(t);
            m
        } else {
            Vec::new()
        };

        let first_enemy_idx = self.units.len();
        let enemy_count = new_enemies.len();
        for mut enemy in new_enemies {
            monster_update_combat_details(&mut enemy);
            self.units.push(enemy);
        }

        let enemy_indices: Vec<UnitIdx> = (first_enemy_idx..first_enemy_idx + enemy_count).collect();
        for &i in &enemy_indices {
            let t = self.simulation_time;
            self.units[i].reset(t);
            let hrid = self.units[i].hrid.clone();
            self.sim_result.update_time_spent_alive(&hrid, true, self.simulation_time);
        }
        self.enemy_indices = if enemy_indices.is_empty() { None } else { Some(enemy_indices) };

        self.event_queue.clear_events_of_type(crate::combatsimulator::events::ENRAGE_TICK);
        self.event_queue.add_event(CombatEvent {
            time: self.simulation_time + ENRAGE_TICK_INTERVAL,
            kind: EventKind::EnrageTick { encounter_time: ENRAGE_TICK_INTERVAL },
        });
        self.enrage_begin_time = self.simulation_time;
        // NOTE: do NOT clear_events_of_type(ABILITY_CAST_END) here.
        // JS leaves the equivalent line commented out (see checkEncounterEnd) so that
        // a player's in-flight ability cast survives a wave transition and resolves
        // normally against the next wave. Clearing it unconditionally here was
        // silently destroying mid-cast player abilities (e.g. water_strike) on every
        // wave clear, forcing a fallback to auto-attack that JS never exhibits.
        // Stale ENEMY-indexed events were already removed above via
        // clear_events_for_unit(enemy_idx) in check_encounter_end(), so this is safe.

        self.check_triggers();
        self.start_attacks();
    }

    fn start_attacks(&mut self) {
        let mut indices: Vec<UnitIdx> = (0..self.num_players).collect();
        if let Some(ref ei) = self.enemy_indices.clone() {
            indices.extend_from_slice(ei);
        }
        for &i in &indices {
            if self.units[i].combat_details.current_hitpoints > 0 {
                self.add_next_attack_event(i);
            }
        }
    }

    // -- Parry -----------------------------------------------------------------

    fn check_parry(&self, target_indices: &[UnitIdx]) -> Option<UnitIdx> {
        let mut rng = rand::thread_rng();
        let mut parry_units: Vec<UnitIdx> = target_indices.iter()
            .copied()
            .filter(|&i| self.units[i].combat_details.current_hitpoints > 0
                && self.units[i].combat_details.combat_stats.parry > 0.0)
            .collect();
        if parry_units.is_empty() { return None; }

        if self.guild_trial_mode {
            // Each incoming attack can receive at most 5 parry attempts, one per
            // parry-capable unit (e.g. Regal Sword users); every eligible unit up
            // to that cap rolls independently and the first success cancels the
            // whole attack.
            parry_units.shuffle(&mut rng);
            return parry_units.into_iter()
                .take(5)
                .find(|&u| self.units[u].combat_details.combat_stats.parry > rng.gen::<f64>());
        }

        let u = parry_units[rng.gen_range(0..parry_units.len())];
        if self.units[u].combat_details.combat_stats.parry > rng.gen::<f64>() { Some(u) } else { None }
    }

    // -- Auto attack -----------------------------------------------------------

    fn process_auto_attack(&mut self, source_idx: UnitIdx) {
        if std::env::var("MWI_DEBUG_AUTOATTACK").is_ok() {
            let hrid = self.units[source_idx].hrid.clone();
            if hrid == "player1" {
                eprintln!("[{:.1}s] process_auto_attack ENTERED for player1 (idx={}) is_player={} current_hp={}",
                    self.simulation_time as f64 / 1e9, source_idx, self.units[source_idx].is_player,
                    self.units[source_idx].combat_details.current_hitpoints);
            }
        }
        let mut rng = rand::thread_rng();
        let target_indices: Vec<UnitIdx> = if self.units[source_idx].is_player {
            match &self.enemy_indices { Some(ei) => ei.clone(), None => return }
        } else {
            (0..self.num_players).collect()
        };

        let alive_targets: Vec<UnitIdx> = target_indices.iter().copied()
            .filter(|&i| self.units[i].combat_details.current_hitpoints > 0)
            .collect();

        if alive_targets.is_empty() {
            if !self.check_encounter_end() { self.add_next_attack_event(source_idx); }
            return;
        }

        let mut i = 0;
        while i < alive_targets.len() {
            // Threat-based target selection for monsters vs multiple players
            let mut target_idx = alive_targets[i];
            if !self.units[source_idx].is_player && alive_targets.len() > 1 {
                let total_threat: f64 = alive_targets.iter()
                    .map(|&j| self.units[j].combat_details.combat_stats.threat).sum();
                let rv = rng.gen::<f64>() * total_threat;
                let mut cum = 0.0;
                for &j in &alive_targets {
                    cum += self.units[j].combat_details.combat_stats.threat;
                    if rv < cum { target_idx = j; break; }
                }
            }

            let mut actual_source = source_idx;
            let mut actual_target = target_idx;
            let parry = self.check_parry(&alive_targets);
            if let Some(p) = parry { actual_target = actual_source; actual_source = p; }

            let attack_result = {
                let (src, tgt) = borrow_two_mut(&mut self.units, actual_source, actual_target);
                CombatUtilities::process_attack(src, tgt, None)
            };

            let is_dungeon = self.zone.as_ref().map(|z| z.is_dungeon).unwrap_or(false);
            if is_dungeon && self.units[actual_target].is_player && attack_result.did_hit && attack_result.damage_done > 0 {
                let log = self.build_combat_log(&self.units[actual_source].hrid.clone(), "autoAttack", actual_target, attack_result.damage_done);
                self.add_to_wipe_logs(log);
            }

            let mayhem = self.units[source_idx].combat_details.combat_stats.mayhem > rng.gen::<f64>();

            if attack_result.did_hit && self.units[actual_source].combat_details.combat_stats.curse > 0.0 {
                self.apply_curse(actual_source, actual_target);
            }
            if self.units[actual_source].combat_details.combat_stats.fury > 0.0 {
                self.apply_fury(actual_source, attack_result.did_hit);
            }
            if self.units[actual_target].combat_details.combat_stats.weaken > 0.0 {
                self.apply_weaken(actual_source, actual_target);
            }

            if !mayhem || (mayhem && attack_result.did_hit) || (mayhem && i == alive_targets.len() - 1) {
                let atk_type = if parry.is_some() { "parry" } else { "autoAttack" };
                if self.units[actual_source].hrid == "player1" && std::env::var("MWI_DEBUG_AUTOATTACK").is_ok() {
                    eprintln!("[{:.1}s] ATTRIBUTION: actual_source=player1 (idx={}) actual_target={} parry={:?} original_source_idx={} did_hit={} dmg={}",
                        self.simulation_time as f64 / 1e9, actual_source, self.units[actual_target].hrid,
                        parry, source_idx, attack_result.did_hit, attack_result.damage_done);
                }
                let src_hrid = self.units[actual_source].hrid.clone();
                let tgt_hrid = self.units[actual_target].hrid.clone();
                let dmg_str = if attack_result.did_hit { attack_result.damage_done.to_string() } else { "miss".to_string() };
                self.sim_result.add_attack(&src_hrid, &tgt_hrid, atk_type, &dmg_str);
                if attack_result.did_hit && attack_result.damage_done > 0 {
                    if self.units[actual_source].is_player {
                        self.sim_result.add_player_damage_dealt(&src_hrid, attack_result.damage_done);
                        self.sim_result.add_player_damage_dealt_by_ability(&src_hrid, "autoAttack", attack_result.damage_done);
                        if attack_result.debuff_damage > 0 {
                            let total_dt = self.units[actual_target].combat_details.combat_stats.damage_taken;
                            for (unique, buff) in &self.units[actual_target].combat_buffs.clone() {
                                if buff.type_hrid == "/buff_types/damage_taken" && buff.flat_boost > 0.0 {
                                    let share = (attack_result.debuff_damage as f64 * buff.flat_boost / total_dt).round() as i64;
                                    self.sim_result.add_debuff_damage(&src_hrid, unique, share);
                                }
                            }
                        }
                        for (unique, contribution) in &attack_result.resist_debuff_damage {
                            self.sim_result.add_debuff_damage(&src_hrid, unique, *contribution);
                        }
                    }
                    if self.units[actual_target].is_player {
                        self.sim_result.add_player_damage_taken(&tgt_hrid, attack_result.damage_done);
                        self.sim_result.add_player_damage_taken_by_source(&tgt_hrid, &src_hrid, attack_result.damage_done);
                        self.sim_result.add_player_damage_taken_by_ability(&tgt_hrid, "autoAttack", attack_result.damage_done);
                    }
                }
            }

            if attack_result.life_steal_heal > 0 {
                let unit = self.units[actual_source].clone();
                self.sim_result.add_hitpoints_gained(&unit, "lifesteal", attack_result.life_steal_heal);
            }
            if attack_result.mana_leech_mana > 0 {
                let unit = self.units[actual_source].clone();
                self.sim_result.add_manapoints_gained(&unit, "manaLeech", attack_result.mana_leech_mana);
            }
            if attack_result.thorn_damage_done > 0 {
                let tgt_hrid = self.units[actual_target].hrid.clone();
                let src_hrid = self.units[actual_source].hrid.clone();
                self.sim_result.add_attack(&tgt_hrid, &src_hrid, &attack_result.thorn_type, &attack_result.thorn_damage_done.to_string());
                if self.units[actual_target].is_player {
                    self.sim_result.add_player_damage_dealt(&tgt_hrid, attack_result.thorn_damage_done);
                    self.sim_result.add_player_damage_dealt_by_ability(&tgt_hrid, &attack_result.thorn_type, attack_result.thorn_damage_done);
                }
                if self.units[actual_source].is_player {
                    self.sim_result.add_player_damage_taken(&src_hrid, attack_result.thorn_damage_done);
                    self.sim_result.add_player_damage_taken_by_source(&src_hrid, &tgt_hrid, attack_result.thorn_damage_done);
                    self.sim_result.add_player_damage_taken_by_ability(&src_hrid, &attack_result.thorn_type, attack_result.thorn_damage_done);
                }
                if is_dungeon && self.units[actual_source].is_player {
                    let log = self.build_combat_log(&tgt_hrid, &attack_result.thorn_type, actual_source, attack_result.thorn_damage_done);
                    self.add_to_wipe_logs(log);
                }
            }
            if self.units[actual_target].combat_details.combat_stats.retaliation > 0.0 {
                let tgt_hrid = self.units[actual_target].hrid.clone();
                let src_hrid = self.units[actual_source].hrid.clone();
                let dmg_str = if attack_result.retaliation_damage_done > 0 { attack_result.retaliation_damage_done.to_string() } else { "miss".to_string() };
                self.sim_result.add_attack(&tgt_hrid, &src_hrid, "retaliation", &dmg_str);
                if attack_result.retaliation_damage_done > 0 {
                    if self.units[actual_target].is_player {
                        self.sim_result.add_player_damage_dealt(&tgt_hrid, attack_result.retaliation_damage_done);
                        self.sim_result.add_player_damage_dealt_by_ability(&tgt_hrid, "retaliation", attack_result.retaliation_damage_done);
                    }
                    if self.units[actual_source].is_player {
                        self.sim_result.add_player_damage_taken(&src_hrid, attack_result.retaliation_damage_done);
                        self.sim_result.add_player_damage_taken_by_source(&src_hrid, &tgt_hrid, attack_result.retaliation_damage_done);
                        self.sim_result.add_player_damage_taken_by_ability(&src_hrid, "retaliation", attack_result.retaliation_damage_done);
                    }
                }
                if is_dungeon && attack_result.retaliation_damage_done > 0 && self.units[actual_source].is_player {
                    let log = self.build_combat_log(&tgt_hrid, "retaliation", actual_source, attack_result.retaliation_damage_done);
                    self.add_to_wipe_logs(log);
                }
            }

            if self.units[actual_target].combat_details.current_hitpoints == 0 {
                if self.units[actual_target].is_player {
                    //eprintln!("[KILL]  {} -> {} (auto) for {} dmg",
                    //    self.units[actual_source].hrid, self.units[actual_target].hrid, attack_result.damage_done);
                }
                self.on_unit_died(actual_target);
            }
            if self.units[actual_source].combat_details.current_hitpoints == 0
                && (attack_result.thorn_damage_done != 0 || attack_result.retaliation_damage_done != 0)
            {
                self.on_unit_died(actual_source);
                break;
            }

            if mayhem && !attack_result.did_hit { i += 1; continue; }

            let pierce = self.units[source_idx].combat_details.combat_stats.pierce;
            if !attack_result.did_hit || parry.is_some() || pierce <= rng.gen::<f64>() { break; }
            i += 1;
        }

        if !self.check_encounter_end() {
            self.add_next_attack_event(source_idx);
        }
    }

    // -- Encounter end check ---------------------------------------------------

    fn on_unit_died(&mut self, idx: UnitIdx) {
        self.event_queue.clear_events_for_unit(idx);
        let unit = self.units[idx].clone();
        if unit.is_player {
            let wave = self.zone.as_ref().map(|z| z.encounters_killed).unwrap_or(0);
            //eprintln!("[DEATH] {} died at t={}s wave={}", unit.hrid, self.simulation_time / 1_000_000_000, wave);
        }
        self.sim_result.add_death(&unit);
        if !unit.is_player {
            self.sim_result.update_time_spent_alive(&unit.hrid, false, self.simulation_time);
        }
    }

    fn check_encounter_end(&mut self) -> bool {
        // Update experience rates for freshly-killed enemies
        if let Some(ei) = self.enemy_indices.clone() {
            for &i in &ei {
                if self.units[i].combat_details.current_hitpoints <= 0 && self.units[i].experience_rate == 0.0 {
                    let alive_dur = (self.simulation_time - self.enrage_begin_time).min(self.units[i].enrage_time);
                    self.units[i].experience_rate = if self.units[i].enrage_time > 0 {
                        1.0 + alive_dur as f64 / self.units[i].enrage_time as f64
                    } else { 1.0 };
                }
            }
        }

        let mut encounter_ended = false;
        let is_dungeon = self.zone.as_ref().map(|z| z.is_dungeon).unwrap_or(false);

        let all_enemies_dead = self.enemy_indices.as_ref()
            .map(|ei| ei.iter().all(|&i| self.units[i].combat_details.current_hitpoints <= 0))
            .unwrap_or(false);

        if all_enemies_dead && self.enemy_indices.is_some() {
            let ei = self.enemy_indices.clone().unwrap();
            // Clear ALL events that reference enemy unit indices before truncating self.units.
            // AutoAttack alone is not enough -- CheckBuffExpiration, StunExpiration,
            // DamageOverTime, etc. all hold raw UnitIdx values that become stale after
            // units.truncate(num_players) in start_new_encounter().
            for &enemy_idx in &ei {
                self.event_queue.clear_events_for_unit(enemy_idx);
            }
            self.event_queue.clear_events_of_type(crate::combatsimulator::events::AUTO_ATTACK);
            self.event_queue.add_event(CombatEvent {
                time: self.simulation_time + ENEMY_RESPAWN_INTERVAL,
                kind: EventKind::EnemyRespawn,
            });

            let total_exp: f64 = ei.iter()
                .map(|&i| self.units[i].experience * self.units[i].experience_rate)
                .sum();
            let exp_per_player = total_exp / self.num_players as f64;
            for j in 0..self.num_players {
                let unit = self.units[j].clone();
                self.sim_result.add_experience_gain(&unit, exp_per_player);
            }

            if is_dungeon {
                let enc = self.zone.as_ref().unwrap().encounters_killed;
                let wave_name = format!("#{}", enc - 1);
                self.sim_result.update_time_spent_alive(&wave_name, false, self.simulation_time);
                let max_waves = self.zone.as_ref().unwrap().dungeon_spawn_info["maxWaves"].as_i64().unwrap_or(0) as i32;
                if enc > max_waves {
                    self.sim_result.update_dungeon_finish("#1", self.simulation_time);
                    self.sim_result.last_dungeon_finish_time = self.simulation_time;
                    self.sim_result.dungeon_attempt_won = Some(true);
                }
            }

            self.sim_result.add_encounter_end();
            self.sim_result.last_encounter_finish_time = self.simulation_time;
            self.enemy_indices = None;
            encounter_ended = true;
        }

        // Player deaths
        for i in 0..self.num_players {
            if self.units[i].combat_details.current_hitpoints <= 0 {
                let hrid = self.units[i].hrid.clone();
                if !is_dungeon && !self.event_queue.contains_event_of_type_and_hrid(
                    crate::combatsimulator::events::PLAYER_RESPAWN, &hrid)
                {
                    self.event_queue.add_event(CombatEvent {
                        time: self.simulation_time + PLAYER_RESPAWN_INTERVAL,
                        kind: EventKind::PlayerRespawn { hrid: hrid.clone() },
                    });
                }
                let unit = self.units[i].clone();
                self.sim_result.add_ran_out_of_mana_count(&unit, false, self.simulation_time);
            }
        }

        let all_players_dead = (0..self.num_players)
            .all(|i| self.units[i].combat_details.current_hitpoints <= 0);

        if all_players_dead {
            if is_dungeon {
                let wave = self.zone.as_ref().map(|z| z.encounters_killed - 1).unwrap_or(0);
                self.save_wipe_logs_to_sim_result(wave);
                self.wipe_log_index = 0;
                self.wipe_log_count = 0;
                self.sim_result.dungeon_attempt_won = Some(false);

                if let Some(ei) = self.enemy_indices.clone() {
                    let total_max: i64 = ei.iter().map(|&i| self.units[i].combat_details.max_hitpoints).sum();
                    let total_cur: i64 = ei.iter()
                        .map(|&i| self.units[i].combat_details.current_hitpoints.max(0))
                        .sum();
                    if total_max > 0 {
                        self.sim_result.enemy_hp_pct_remaining =
                            Some(total_cur as f64 / total_max as f64 * 100.0);
                    }
                }

                for type_str in &[
                    crate::combatsimulator::events::AUTO_ATTACK,
                    crate::combatsimulator::events::ABILITY_CAST_END,
                    crate::combatsimulator::events::DAMAGE_OVER_TIME,
                    crate::combatsimulator::events::CONSUMABLE_TICK,
                    crate::combatsimulator::events::REGEN_TICK,
                    crate::combatsimulator::events::ENRAGE_TICK,
                    crate::combatsimulator::events::STUN_EXPIRATION,
                    crate::combatsimulator::events::BLIND_EXPIRATION,
                    crate::combatsimulator::events::SILENCE_EXPIRATION,
                    crate::combatsimulator::events::AWAIT_COOLDOWN,
                ] {
                    self.event_queue.clear_events_of_type(type_str);
                }
                self.enemy_indices = None;
                self.event_queue.add_event(CombatEvent {
                    time: self.simulation_time + RESTART_INTERVAL,
                    kind: EventKind::CombatStart,
                });
            } else {
                self.event_queue.clear_events_of_type(crate::combatsimulator::events::AUTO_ATTACK);
                self.event_queue.clear_events_of_type(crate::combatsimulator::events::ABILITY_CAST_END);
            }
            encounter_ended = true;
            self.all_players_dead = true;
        }

        // Labyrinth timeout
        let laby_done = if let Some(ref mut laby) = self.labyrinth {
            laby.check_timeout(self.simulation_time) || encounter_ended
        } else { false };

        if laby_done {
            self.enemy_indices = None;
            encounter_ended = true;
            self.event_queue.clear();
            self.event_queue.add_event(CombatEvent { time: self.simulation_time, kind: EventKind::CombatStart });
        }

        encounter_ended
    }

    // -- Scheduling next attack ------------------------------------------------

    fn add_next_attack_event(&mut self, source_idx: UnitIdx) {
        // Don't double-schedule
        let already = self.event_queue.get_matching(|e| {
            matches!(&e.kind, EventKind::AutoAttack { source } if *source == source_idx)
            || matches!(&e.kind, EventKind::AbilityCastEnd { source, .. } if *source == source_idx)
        }).is_some();
        if already { return; }

        let is_player = self.units[source_idx].is_player;
        let is_stunned = self.units[source_idx].is_stunned;
        let is_silenced = self.units[source_idx].is_silenced;
        let haste = self.units[source_idx].combat_details.combat_stats.ability_haste;
        let sim_time = self.simulation_time;

        let (targets_opt, friendlies, enemies_opt) = if is_player {
            let ei = self.enemy_indices.clone();
            (ei.clone(), (0..self.num_players).collect::<Vec<_>>(), ei)
        } else {
            let ei = self.enemy_indices.clone().unwrap_or_default();
            let players: Vec<_> = (0..self.num_players).collect();
            (Some(players.clone()), ei, Some(players))
        };

        // Try abilities
        let mut used_ability = false;
        let mut skip_next = false;

        for abi in 0..5usize {
            if used_ability || skip_next { break; }
            let can_trigger = if let Some(ref ability) = self.units[source_idx].abilities[abi] {
                if is_stunned || is_silenced { false } else {
                    let mut cd = ability.cooldown_duration as f64;
                    if haste > 0.0 { cd = cd * 100.0 / (100.0 + haste); }
                    ability.last_used.saturating_add(cd as i64) <= sim_time
                        && self.evaluate_triggers(source_idx, abi, &targets_opt, &friendlies, &enemies_opt.clone().unwrap_or_default())
                }
            } else { false };

            if can_trigger {
                if !self.can_use_ability(source_idx, abi, true) {
                    skip_next = true;
                } else {
                    let cast_dur = {
                        let ab = self.units[source_idx].abilities[abi].as_ref().unwrap();
                        let dur = ab.cast_duration as f64 / (1.0 + self.units[source_idx].combat_details.combat_stats.cast_speed);
                        dur as i64
                    };
                    self.event_queue.add_event(CombatEvent {
                        time: sim_time + cast_dur,
                        kind: EventKind::AbilityCastEnd { source: source_idx, ability_idx: abi },
                    });
                    used_ability = true;
                }
            }
        }

        if used_ability { self.units[source_idx].is_out_of_mana = false; return; }
        if targets_opt.is_none() {
            if self.units[source_idx].is_player && std::env::var("MWI_DEBUG_AUTOATTACK").is_ok() {
                eprintln!("[{:.1}s] {} fell through to RETURN (no targets, ability loop never resolved) skip_next={}",
                    sim_time as f64 / 1e9, self.units[source_idx].hrid, skip_next);
            }
            return;
        }

        if self.units[source_idx].is_player && std::env::var("MWI_DEBUG_AUTOATTACK").is_ok() {
            eprintln!("[{:.1}s] {} falling through to AUTO-ATTACK. skip_next={} current_hp={} is_blinded={} is_silenced={} is_stunned={}",
                sim_time as f64 / 1e9, self.units[source_idx].hrid, skip_next,
                self.units[source_idx].combat_details.current_hitpoints,
                self.units[source_idx].is_blinded, self.units[source_idx].is_silenced, self.units[source_idx].is_stunned);
            for (i, ab) in self.units[source_idx].abilities.iter().enumerate() {
                if let Some(ab) = ab {
                    let mut cd = ab.cooldown_duration as f64;
                    if haste > 0.0 { cd = cd * 100.0 / (100.0 + haste); }
                    let cd_ready = ab.last_used.saturating_add(cd as i64) <= sim_time;
                    let trig_pass = self.evaluate_triggers(source_idx, i, &targets_opt, &friendlies, &enemies_opt.clone().unwrap_or_default());
                    eprintln!("    slot {}: {} cd_ready={} trig_pass={} mana_cost={} cur_mana={} last_used={} sim_time={}",
                        i, ab.hrid, cd_ready, trig_pass, ab.mana_cost,
                        self.units[source_idx].combat_details.current_manapoints,
                        ab.last_used, sim_time);
                }
            }
        }

        if !self.units[source_idx].is_blinded {
            let interval = self.units[source_idx].combat_details.combat_stats.attack_interval as i64;
            self.event_queue.add_event(CombatEvent {
                time: sim_time + interval,
                kind: EventKind::AutoAttack { source: source_idx },
            });
        } else {
            self.units[source_idx].is_out_of_mana = true;
        }
    }

    // -- Trigger evaluation ----------------------------------------------------

    fn evaluate_triggers(
        &self, source_idx: UnitIdx, ability_idx: usize,
        targets: &Option<Vec<UnitIdx>>, friendlies: &[UnitIdx], enemies: &[UnitIdx],
    ) -> bool {
        let ability = match &self.units[source_idx].abilities[ability_idx] {
            Some(a) => a.clone(),
            None => return false,
        };
        if ability.triggers.is_empty() { return true; }

        let target_idx = targets.as_ref().and_then(|t| {
            t.iter().find(|&&i| self.units[i].combat_details.current_hitpoints > 0).copied()
        });

        for trigger in &ability.triggers {
            let is_single = *crate::combatsimulator::data::combat_trigger_dependency_map()
                .get(&trigger.dependency_hrid).unwrap_or(&false);
            let ok = if is_single {
                self.eval_trigger_single(trigger, source_idx, target_idx)
            } else {
                self.eval_trigger_multi(trigger, friendlies, enemies)
            };
            if !ok { return false; }
        }
        true
    }

    fn eval_trigger_single(&self, trigger: &crate::combatsimulator::trigger::Trigger, source_idx: UnitIdx, target_idx: Option<UnitIdx>) -> bool {
        let dep = match trigger.dependency_hrid.as_str() {
            "/combat_trigger_dependencies/self" => source_idx,
            "/combat_trigger_dependencies/targeted_enemy" => match target_idx {
                Some(i) => i,
                None => return false,
            },
            _ => return false,
        };
        let val = self.get_trigger_condition_value(trigger, dep);
        compare_trigger(&trigger.comparator_hrid, val, trigger.value)
    }

    fn eval_trigger_multi(&self, trigger: &crate::combatsimulator::trigger::Trigger, friendlies: &[UnitIdx], enemies: &[UnitIdx]) -> bool {
        let units = match trigger.dependency_hrid.as_str() {
            "/combat_trigger_dependencies/all_allies"  => friendlies,
            "/combat_trigger_dependencies/all_enemies" => enemies,
            _ => return false,
        };
        if units.is_empty() { return false; }
        let alive: Vec<UnitIdx> = units.iter().copied()
            .filter(|&i| self.units[i].combat_details.current_hitpoints > 0).collect();

        let val = match trigger.condition_hrid.as_str() {
            "/combat_trigger_conditions/number_of_active_units" => alive.len() as f64,
            "/combat_trigger_conditions/number_of_dead_units"   => (units.len() - alive.len()) as f64,
            "/combat_trigger_conditions/lowest_hp_percentage"   => {
                alive.iter().map(|&i| {
                    self.units[i].combat_details.current_hitpoints as f64
                        / self.units[i].combat_details.max_hitpoints as f64
                }).fold(2.0f64, f64::min) * 100.0
            },
            _ => alive.iter().map(|&i| self.get_trigger_condition_value(trigger, i)).sum(),
        };
        compare_trigger(&trigger.comparator_hrid, val, trigger.value)
    }

    fn get_trigger_condition_value(&self, trigger: &crate::combatsimulator::trigger::Trigger, unit_idx: UnitIdx) -> f64 {
        let unit = &self.units[unit_idx];
        let sim_time = self.simulation_time;
        match trigger.condition_hrid.as_str() {
            "/combat_trigger_conditions/current_hp"     => unit.combat_details.current_hitpoints as f64,
            "/combat_trigger_conditions/current_mp"     => unit.combat_details.current_manapoints as f64,
            "/combat_trigger_conditions/missing_hp"     => (unit.combat_details.max_hitpoints - unit.combat_details.current_hitpoints) as f64,
            "/combat_trigger_conditions/missing_mp"     => (unit.combat_details.max_manapoints - unit.combat_details.current_manapoints) as f64,
            "/combat_trigger_conditions/stun_status"    => if unit.is_stunned || unit.stun_expire_time == Some(sim_time) { 1.0 } else { 0.0 },
            "/combat_trigger_conditions/blind_status"   => if unit.is_blinded || unit.blind_expire_time == Some(sim_time) { 1.0 } else { 0.0 },
            "/combat_trigger_conditions/silence_status" => if unit.is_silenced || unit.silence_expire_time == Some(sim_time) { 1.0 } else { 0.0 },
            cond => {
                // Buff-based conditions
                let buff_prefix = if matches!(cond,
                    "/combat_trigger_conditions/critical_aura" | "/combat_trigger_conditions/critical_coffee"
                    | "/combat_trigger_conditions/intelligence_coffee" | "/combat_trigger_conditions/stamina_coffee"
                    | "/combat_trigger_conditions/elemental_affinity" | "/combat_trigger_conditions/fury"
                    | "/combat_trigger_conditions/guardian_aura" | "/combat_trigger_conditions/insanity"
                    | "/combat_trigger_conditions/spike_shell" | "/combat_trigger_conditions/toxic_pollen"
                    | "/combat_trigger_conditions/invincible" | "/combat_trigger_conditions/mystic_aura"
                    | "/combat_trigger_conditions/pestilent_shot" | "/combat_trigger_conditions/smoke_burst"
                    | "/combat_trigger_conditions/speed_aura" | "/combat_trigger_conditions/toughness"
                    | "/combat_trigger_conditions/enrage"
                ) { true } else { false };

                let buff_hrid = format!("/buff_uniques{}", &cond[cond.rfind('/').unwrap_or(0)..]);
                if buff_prefix {
                    if unit.has_buff_starting_with(&buff_hrid) { 1.0 } else { 0.0 }
                } else {
                    if unit.combat_buffs.contains_key(&buff_hrid) { 1.0 } else { 0.0 }
                }
            }
        }
    }

    // -- Ability execution -----------------------------------------------------

    fn can_use_ability(&mut self, source_idx: UnitIdx, ability_idx: usize, oom_check: bool) -> bool {
        if self.units[source_idx].combat_details.current_hitpoints <= 0 { return false; }
        let mana_cost = self.units[source_idx].abilities[ability_idx].as_ref().map(|a| a.mana_cost).unwrap_or(0);
        if self.units[source_idx].combat_details.current_manapoints < mana_cost {
            if self.units[source_idx].is_player && oom_check {
                let unit = self.units[source_idx].clone();
                self.sim_result.add_ran_out_of_mana_count(&unit, true, self.simulation_time);
            }
            return false;
        }
        if self.units[source_idx].is_player && oom_check {
            let unit = self.units[source_idx].clone();
            self.sim_result.add_ran_out_of_mana_count(&unit, false, self.simulation_time);
        }
        true
    }

    fn try_use_ability(&mut self, source_idx: UnitIdx, ability_idx: usize) -> bool {
        if !self.can_use_ability(source_idx, ability_idx, true) { return false; }

        let ability = match self.units[source_idx].abilities[ability_idx].clone() {
            Some(a) => a,
            None => return false,
        };


        if self.units[source_idx].is_player {
            let cost = ability.mana_cost;
            let hrid = ability.hrid.clone();
            *self.units[source_idx].ability_mana_costs.entry(hrid).or_insert(0) += cost;
        }
        self.units[source_idx].combat_details.current_manapoints -= ability.mana_cost;

        let haste = self.units[source_idx].combat_details.combat_stats.ability_haste;
        let mut cd = ability.cooldown_duration as f64;
        if haste > 0.0 { cd = cd * 100.0 / (100.0 + haste); }
        let sim_time = self.simulation_time;
        if let Some(ab) = self.units[source_idx].abilities[ability_idx].as_mut() {
            ab.last_used = sim_time;
        }

        let mut rng = rand::thread_rng();
        let blaze_chance = self.units[source_idx].combat_details.combat_stats.blaze;
        let bloom_chance = self.units[source_idx].combat_details.combat_stats.bloom;

        let mut to_execute: Vec<Ability> = vec![ability.clone()];
        if blaze_chance > 0.0 && rng.gen::<f64>() < blaze_chance {
            to_execute.push(Ability::new("blaze".to_string(), 1, None));
        }
        if bloom_chance > 0.0 && rng.gen::<f64>() < bloom_chance {
            to_execute.push(Ability::new("bloom".to_string(), 1, None));
        }

        for exec_ab in to_execute {
            for effect in exec_ab.ability_effects.clone() {
                match effect.effect_type.as_str() {
                    "/ability_effect_types/buff"     => self.process_ability_buff_effect(source_idx, &exec_ab, &effect),
                    "/ability_effect_types/damage"   => self.process_ability_damage_effect(source_idx, &exec_ab, &effect),
                    "/ability_effect_types/heal"     => self.process_ability_heal_effect(source_idx, &exec_ab, &effect),
                    "/ability_effect_types/spend_hp" => {
                        let spent = CombatUtilities::process_spend_hp(&mut self.units[source_idx], &effect);
                        let unit = self.units[source_idx].clone();
                        self.sim_result.add_hitpoints_spent(&unit, &exec_ab.hrid, spent);
                    },
                    "/ability_effect_types/revive"   => self.process_ability_revive_effect(source_idx, &exec_ab, &effect),
                    "/ability_effect_types/promote"  => {
                        let promotions = ["/monsters/enchanted_rook", "/monsters/enchanted_knight", "/monsters/enchanted_bishop"];
                        let pick = promotions[rng.gen_range(0..promotions.len())];
                        let tier = self.units[source_idx].monster_difficulty_tier;
                        let num_players = self.units[source_idx].monster_num_players;
                        let mut new_monster = MonsterBuilder::new(pick.to_string(), tier, 0, num_players);
                        monster_update_combat_details(&mut new_monster);
                        self.units[source_idx] = new_monster;
                        self.add_next_attack_event(source_idx);
                    },
                    other => eprintln!("Unsupported effect type: {}", other),
                }
            }
        }

        // Ripple: chance to restore 10 MP and reduce cooldowns by 2s
        let ripple = self.units[source_idx].combat_details.combat_stats.ripple;
        if ripple > 0.0 && rng.gen::<f64>() < ripple {
            let mp_added = self.units[source_idx].add_manapoints(10);
            let unit = self.units[source_idx].clone();
            self.sim_result.add_manapoints_gained(&unit, "ripple", mp_added);
            let now = self.simulation_time;
            for ab in self.units[source_idx].abilities.iter_mut().flatten() {
                let remaining = ab.last_used.saturating_add(ab.cooldown_duration).saturating_sub(now);
                if remaining > 0 {
                    ab.last_used = ab.last_used.saturating_sub(ONE_SECOND * 2).max(now - ab.cooldown_duration);
                }
            }
        }

        if self.units[source_idx].combat_details.current_hitpoints == 0 {
            self.on_unit_died(source_idx);
        } else {
            self.add_next_attack_event(source_idx);
        }
        self.check_encounter_end();
        true
    }

    // -- Ability effects -------------------------------------------------------

    fn process_ability_buff_effect(&mut self, source_idx: UnitIdx, ability: &Ability, effect: &crate::combatsimulator::ability::AbilityEffect) {
        let targets: Vec<UnitIdx> = match effect.target_type.as_str() {
            "allAllies" => if self.units[source_idx].is_player { (0..self.num_players).collect() }
                          else { self.enemy_indices.clone().unwrap_or_default() },
            "self"      => vec![source_idx],
            other => { eprintln!("Unsupported buff target: {}", other); return; },
        };

        let buffs = effect.buffs.clone().unwrap_or_default();
        for &tgt in &targets {
            if self.units[tgt].combat_details.current_hitpoints <= 0 { continue; }
            for buff in &buffs {
                let mut cur = buff.clone();
                if ability.is_special_ability && !buff.multiplier_for_skill_hrid.is_empty() && buff.multiplier_per_skill_level > 0.0 {
                    let skill = buff.multiplier_for_skill_hrid.split('/').last().unwrap_or("");
                    let level = get_skill_level(&self.units[source_idx], skill);
                    let mult = 1.0 + level * buff.multiplier_per_skill_level;
                    cur.flat_boost *= mult;
                    cur.ratio_boost *= mult;
                }
                let t = self.simulation_time;
                self.units[tgt].add_buff(cur.clone(), t, source_idx);
                let dur = cur.duration;
                self.event_queue.add_event(CombatEvent {
                    time: self.simulation_time + dur,
                    kind: EventKind::CheckBuffExpiration { source: tgt },
                });
            }
        }
    }

    fn process_ability_damage_effect(&mut self, source_idx: UnitIdx, ability: &Ability, effect: &crate::combatsimulator::ability::AbilityEffect) {
        let mut rng = rand::thread_rng();
        let target_pool: Vec<UnitIdx> = match effect.target_type.as_str() {
            "enemy" | "allEnemies" => {
                if self.units[source_idx].is_player { self.enemy_indices.clone().unwrap_or_default() }
                else { (0..self.num_players).collect() }
            },
            other => { eprintln!("Unsupported damage target: {}", other); return; },
        };
        if target_pool.is_empty() { return; }

        let is_dungeon = self.zone.as_ref().map(|z| z.is_dungeon).unwrap_or(false);
        let mut avoid: Vec<UnitIdx> = Vec::new();
        let mut is_skip_parry = false;

        let mut t_idx = 0;
        while t_idx < target_pool.len() {
            let raw_tgt = target_pool[t_idx];
            if self.units[raw_tgt].combat_details.current_hitpoints <= 0 { t_idx += 1; continue; }

            let parry_opt = if !is_skip_parry {
                is_skip_parry = true;
                self.check_parry(&target_pool)
            } else { None };

            if let Some(parry_idx) = parry_opt {
                // Parry reflects back to source
                let ar = {
                    let (psrc, ptgt) = borrow_two_mut(&mut self.units, parry_idx, source_idx);
                    CombatUtilities::process_attack(psrc, ptgt, None)
                };
                let src_hrid = self.units[parry_idx].hrid.clone();
                let tgt_hrid = self.units[source_idx].hrid.clone();
                let dmg_str = if ar.did_hit { ar.damage_done.to_string() } else { "miss".to_string() };
                self.sim_result.add_attack(&src_hrid, &tgt_hrid, "parry", &dmg_str);
                if ar.did_hit && ar.damage_done > 0 {
                    if self.units[parry_idx].is_player {
                        self.sim_result.add_player_damage_dealt(&src_hrid, ar.damage_done);
                        self.sim_result.add_player_damage_dealt_by_ability(&src_hrid, "parry", ar.damage_done);
                    }
                    if self.units[source_idx].is_player {
                        self.sim_result.add_player_damage_taken(&tgt_hrid, ar.damage_done);
                        self.sim_result.add_player_damage_taken_by_source(&tgt_hrid, &src_hrid, ar.damage_done);
                        self.sim_result.add_player_damage_taken_by_ability(&tgt_hrid, "parry", ar.damage_done);
                    }
                }
                if ar.life_steal_heal > 0 {
                    let u = self.units[parry_idx].clone();
                    self.sim_result.add_hitpoints_gained(&u, "lifesteal", ar.life_steal_heal);
                }
                if ar.thorn_damage_done > 0 {
                    self.sim_result.add_attack(&tgt_hrid, &src_hrid, &ar.thorn_type, &ar.thorn_damage_done.to_string());
                }
                if self.units[source_idx].combat_details.current_hitpoints == 0 { self.on_unit_died(source_idx); }
                break;
            }

            // Regular hit
            let alive_pool: Vec<UnitIdx> = target_pool.iter().copied()
                .filter(|&i| self.units[i].combat_details.current_hitpoints > 0 && !avoid.contains(&i))
                .collect();
            if alive_pool.is_empty() { break; }

            let mut actual_target = alive_pool[0];

            // Threat targeting for single-target monster ability vs multiple players
            if !self.units[source_idx].is_player && effect.target_type == "enemy" && alive_pool.len() > 1 {
                let total_threat: f64 = alive_pool.iter().map(|&j| self.units[j].combat_details.combat_stats.threat).sum();
                let rv = rng.gen::<f64>() * total_threat;
                let mut cum = 0.0;
                for &j in &alive_pool {
                    cum += self.units[j].combat_details.combat_stats.threat;
                    if rv < cum { actual_target = j; break; }
                }
            }
            // Always track actual_target in avoid so allEnemies hits each target once
            avoid.push(actual_target);

            let eff_clone = effect.clone();
            let ar = {
                let (src, tgt) = borrow_two_mut(&mut self.units, source_idx, actual_target);
                CombatUtilities::process_attack(src, tgt, Some(&eff_clone))
            };

            if is_dungeon && self.units[actual_target].is_player && ar.did_hit && ar.damage_done > 0 {
                let log = self.build_combat_log(&self.units[source_idx].hrid.clone(), &ability.hrid, actual_target, ar.damage_done);
                self.add_to_wipe_logs(log);
            }
            if ar.hp_drain > 0 {
                let u = self.units[source_idx].clone();
                self.sim_result.add_hitpoints_gained(&u, &ability.hrid, ar.hp_drain);
            }

            if ar.did_hit {
                if let Some(ref buffs) = effect.buffs {
                    for buff in buffs {
                        let t = self.simulation_time;
                        self.units[actual_target].add_buff(buff.clone(), t, source_idx);
                        let dur = buff.duration;
                        self.event_queue.add_event(CombatEvent {
                            time: self.simulation_time + dur,
                            kind: EventKind::CheckBuffExpiration { source: actual_target },
                        });
                    }
                }
            }

            // DoT
            if effect.damage_over_time_ratio > 0.0 && ar.damage_done > 0 {
                let tot_ticks = (effect.damage_over_time_duration / DOT_TICK_INTERVAL) as i32;
                self.event_queue.add_event(CombatEvent {
                    time: self.simulation_time + DOT_TICK_INTERVAL,
                    kind: EventKind::DamageOverTime {
                        source_ref: source_idx, target: actual_target,
                        damage: ar.damage_done as f64 * effect.damage_over_time_ratio,
                        total_ticks: tot_ticks, current_tick: 1,
                        combat_style_hrid: effect.combat_style_hrid.clone(),
                    },
                });
            }

            // Stun / Blind / Silence
            if ar.did_hit {
                let tenacity = self.units[actual_target].combat_details.combat_stats.tenacity;
                if effect.stun_chance > 0.0 && rng.gen::<f64>() < effect.stun_chance * 100.0 / (100.0 + tenacity) {
                    self.units[actual_target].is_stunned = true;
                    let exp = self.simulation_time + effect.stun_duration;
                    self.units[actual_target].stun_expire_time = Some(exp);
                    let at = actual_target;
                    self.event_queue.clear_matching(|e| matches!(&e.kind,
                        EventKind::AutoAttack { source } | EventKind::AbilityCastEnd { source, .. } | EventKind::StunExpiration { source }
                        if *source == at));
                    self.event_queue.add_event(CombatEvent { time: exp, kind: EventKind::StunExpiration { source: actual_target } });
                }
                if effect.blind_chance > 0.0 && rng.gen::<f64>() < effect.blind_chance * 100.0 / (100.0 + tenacity) {
                    self.units[actual_target].is_blinded = true;
                    let exp = self.simulation_time + effect.blind_duration;
                    self.units[actual_target].blind_expire_time = Some(exp);
                    let at = actual_target;
                    self.event_queue.clear_matching(|e| matches!(&e.kind, EventKind::BlindExpiration { source } if *source == at));
                    if self.event_queue.clear_matching(|e| matches!(&e.kind, EventKind::AutoAttack { source } if *source == at)) {
                        self.add_next_attack_event(actual_target);
                    }
                    self.event_queue.add_event(CombatEvent { time: exp, kind: EventKind::BlindExpiration { source: actual_target } });
                }
                if effect.silence_chance > 0.0 && rng.gen::<f64>() < effect.silence_chance * 100.0 / (100.0 + tenacity) {
                    self.units[actual_target].is_silenced = true;
                    let exp = self.simulation_time + effect.silence_duration;
                    self.units[actual_target].silence_expire_time = Some(exp);
                    let at = actual_target;
                    self.event_queue.clear_matching(|e| matches!(&e.kind, EventKind::SilenceExpiration { source } if *source == at));
                    if self.event_queue.clear_matching(|e| matches!(&e.kind, EventKind::AbilityCastEnd { source, .. } if *source == at)) {
                        self.add_next_attack_event(actual_target);
                    }
                    self.event_queue.add_event(CombatEvent { time: exp, kind: EventKind::SilenceExpiration { source: actual_target } });
                }

                if self.units[source_idx].combat_details.combat_stats.curse > 0.0 {
                    self.apply_curse(source_idx, actual_target);
                }
            }
            if self.units[source_idx].combat_details.combat_stats.fury > 0.0 {
                self.apply_fury(source_idx, ar.did_hit);
            }
            if self.units[actual_target].combat_details.combat_stats.weaken > 0.0 {
                self.apply_weaken(source_idx, actual_target);
            }

            let src_hrid = self.units[source_idx].hrid.clone();
            let tgt_hrid = self.units[actual_target].hrid.clone();
            let dmg_str = if ar.did_hit { ar.damage_done.to_string() } else { "miss".to_string() };
            self.sim_result.add_attack(&src_hrid, &tgt_hrid, &ability.hrid, &dmg_str);
            if ar.did_hit && ar.damage_done > 0 {
                if self.units[source_idx].is_player {
                    if src_hrid == "player1" && std::env::var("MWI_DEBUG_AUTOATTACK").is_ok() {
                        eprintln!("[{:.1}s] ABILITY DAMAGE: player1 dealt {} via ability.hrid={:?}",
                            self.simulation_time as f64 / 1e9, ar.damage_done, ability.hrid);
                    }
                    self.sim_result.add_player_damage_dealt(&src_hrid, ar.damage_done);
                    self.sim_result.add_player_damage_dealt_by_ability(&src_hrid, &ability.hrid, ar.damage_done);
                    if ar.debuff_damage > 0 {
                        let total_dt = self.units[actual_target].combat_details.combat_stats.damage_taken;
                        for (unique, buff) in &self.units[actual_target].combat_buffs.clone() {
                            if buff.type_hrid == "/buff_types/damage_taken" && buff.flat_boost > 0.0 {
                                let share = (ar.debuff_damage as f64 * buff.flat_boost / total_dt).round() as i64;
                                self.sim_result.add_debuff_damage(&src_hrid, unique, share);
                            }
                        }
                    }
                    for (unique, contribution) in &ar.resist_debuff_damage {
                        self.sim_result.add_debuff_damage(&src_hrid, unique, *contribution);
                    }
                }
                if self.units[actual_target].is_player {
                    self.sim_result.add_player_damage_taken(&tgt_hrid, ar.damage_done);
                    self.sim_result.add_player_damage_taken_by_source(&tgt_hrid, &src_hrid, ar.damage_done);
                    self.sim_result.add_player_damage_taken_by_ability(&tgt_hrid, &ability.hrid, ar.damage_done);
                }
            }
            if ar.thorn_damage_done > 0 {
                self.sim_result.add_attack(&tgt_hrid, &src_hrid, &ar.thorn_type, &ar.thorn_damage_done.to_string());
                if self.units[actual_target].is_player {
                    self.sim_result.add_player_damage_dealt(&tgt_hrid, ar.thorn_damage_done);
                    self.sim_result.add_player_damage_dealt_by_ability(&tgt_hrid, &ar.thorn_type, ar.thorn_damage_done);
                }
                if self.units[source_idx].is_player {
                    self.sim_result.add_player_damage_taken(&src_hrid, ar.thorn_damage_done);
                    self.sim_result.add_player_damage_taken_by_source(&src_hrid, &tgt_hrid, ar.thorn_damage_done);
                    self.sim_result.add_player_damage_taken_by_ability(&src_hrid, &ar.thorn_type, ar.thorn_damage_done);
                }
                if is_dungeon && self.units[source_idx].is_player {
                    let log = self.build_combat_log(&tgt_hrid, &ar.thorn_type, source_idx, ar.thorn_damage_done);
                    self.add_to_wipe_logs(log);
                }
            }
            if self.units[actual_target].combat_details.combat_stats.retaliation > 0.0 {
                let dmg_str = if ar.retaliation_damage_done > 0 { ar.retaliation_damage_done.to_string() } else { "miss".to_string() };
                self.sim_result.add_attack(&tgt_hrid, &src_hrid, "retaliation", &dmg_str);
                if ar.retaliation_damage_done > 0 {
                    if self.units[actual_target].is_player {
                        self.sim_result.add_player_damage_dealt(&tgt_hrid, ar.retaliation_damage_done);
                        self.sim_result.add_player_damage_dealt_by_ability(&tgt_hrid, "retaliation", ar.retaliation_damage_done);
                    }
                    if self.units[source_idx].is_player {
                        self.sim_result.add_player_damage_taken(&src_hrid, ar.retaliation_damage_done);
                        self.sim_result.add_player_damage_taken_by_source(&src_hrid, &tgt_hrid, ar.retaliation_damage_done);
                        self.sim_result.add_player_damage_taken_by_ability(&src_hrid, "retaliation", ar.retaliation_damage_done);
                    }
                }
                if is_dungeon && ar.retaliation_damage_done > 0 && self.units[source_idx].is_player {
                    let log = self.build_combat_log(&tgt_hrid, "retaliation", source_idx, ar.retaliation_damage_done);
                    self.add_to_wipe_logs(log);
                }
            }

            if self.units[actual_target].combat_details.current_hitpoints == 0 {
                if self.units[actual_target].is_player {
                    let ability_name = ability.hrid.split('/').last().unwrap_or(&ability.hrid);
                    //eprintln!("[KILL]  {} -> {} ({}) for {} dmg",
                    //    self.units[source_idx].hrid, self.units[actual_target].hrid, ability_name, ar.damage_done);
                }
                self.on_unit_died(actual_target);
            }

            if ar.did_hit && effect.pierce_chance > rng.gen::<f64>() { t_idx += 1; continue; }
            if effect.target_type == "enemy" { break; }
            t_idx += 1;
        }
    }

    fn process_ability_heal_effect(&mut self, source_idx: UnitIdx, ability: &Ability, effect: &crate::combatsimulator::ability::AbilityEffect) {
        let targets: Vec<UnitIdx> = match effect.target_type.as_str() {
            "allAllies" => if self.units[source_idx].is_player { (0..self.num_players).collect() }
                          else { self.enemy_indices.clone().unwrap_or_default() },
            "lowestHpAlly" => {
                let pool: Vec<UnitIdx> = if self.units[source_idx].is_player { (0..self.num_players).collect() }
                                         else { self.enemy_indices.clone().unwrap_or_default() };
                let alive: Vec<_> = pool.iter().copied().filter(|&i| self.units[i].combat_details.current_hitpoints > 0).collect();
                if let Some(&best) = alive.iter().min_by(|&&a, &&b| {
                    let pa = self.units[a].combat_details.current_hitpoints as f64 / self.units[a].combat_details.max_hitpoints as f64;
                    let pb = self.units[b].combat_details.current_hitpoints as f64 / self.units[b].combat_details.max_hitpoints as f64;
                    pa.partial_cmp(&pb).unwrap()
                }) { vec![best] } else { vec![] }
            },
            "self" => vec![source_idx],
            other => { eprintln!("Unsupported heal target: {}", other); return; },
        };

        let src_clone = self.units[source_idx].clone();
        for &tgt in &targets {
            if self.units[tgt].combat_details.current_hitpoints <= 0 { continue; }
            let healed = CombatUtilities::process_heal(&src_clone, effect, &mut self.units[tgt]);
            let tgt_unit = self.units[tgt].clone();
            self.sim_result.add_hitpoints_gained(&tgt_unit, &ability.hrid, healed);
        }
    }

    fn process_ability_revive_effect(&mut self, source_idx: UnitIdx, ability: &Ability, effect: &crate::combatsimulator::ability::AbilityEffect) {
        let pool: Vec<UnitIdx> = if self.units[source_idx].is_player { (0..self.num_players).collect() }
                                  else { self.enemy_indices.clone().unwrap_or_default() };
        if let Some(&revive_idx) = pool.iter().find(|&&i| self.units[i].combat_details.current_hitpoints <= 0) {
            let hrid = self.units[revive_idx].hrid.clone();
            self.event_queue.clear_matching(|e| matches!(&e.kind, EventKind::PlayerRespawn { hrid: h } if *h == hrid));
            let t = self.simulation_time;
            self.units[revive_idx].remove_expired_buffs(t);
            let src = self.units[source_idx].clone();
            let healed = CombatUtilities::process_revive(&src, effect, &mut self.units[revive_idx]);
            let tgt = self.units[revive_idx].clone();
            self.sim_result.add_hitpoints_gained(&tgt, &ability.hrid, healed);
            self.add_next_attack_event(revive_idx);
            if !self.units[source_idx].is_player {
                self.sim_result.update_time_spent_alive(&hrid, true, self.simulation_time);
            }
        }
    }

    // -- Periodic events -------------------------------------------------------

    fn process_consumable_tick(&mut self, source_idx: UnitIdx, consumable_hrid: &str, total_ticks: i32, current_tick: i32) {
        // Find the consumable on the unit by hrid
        let consumable_clone = {
            let unit = &self.units[source_idx];
            unit.food.iter().chain(unit.drinks.iter())
                .find(|c| c.as_ref().map(|c| c.hrid.as_str()) == Some(consumable_hrid))
                .and_then(|c| c.clone())
        };
        let consumable = match consumable_clone { Some(c) => c, None => return };

        if consumable.hitpoint_restore > 0 {
            let tick = CombatUtilities::calculate_tick_value(consumable.hitpoint_restore, total_ticks, current_tick);
            let added = self.units[source_idx].add_hitpoints(tick);
            let unit = self.units[source_idx].clone();
            self.sim_result.add_hitpoints_gained(&unit, &consumable.hrid, added);
        }
        if consumable.manapoint_restore > 0 {
            let tick = CombatUtilities::calculate_tick_value(consumable.manapoint_restore, total_ticks, current_tick);
            let added = self.units[source_idx].add_manapoints(tick);
            let unit = self.units[source_idx].clone();
            self.sim_result.add_manapoints_gained(&unit, &consumable.hrid, added);
            if self.units[source_idx].is_out_of_mana {
                self.event_queue.add_event(CombatEvent {
                    time: self.simulation_time,
                    kind: EventKind::AwaitCooldown { source: source_idx },
                });
            }
        }
        if current_tick < total_ticks {
            self.event_queue.add_event(CombatEvent {
                time: self.simulation_time + HOT_TICK_INTERVAL,
                kind: EventKind::ConsumableTick {
                    source: source_idx, consumable_hrid: consumable_hrid.to_string(),
                    total_ticks, current_tick: current_tick + 1,
                },
            });
        }
    }

    fn process_dot_tick(&mut self, source_ref: UnitIdx, target: UnitIdx, damage: f64, total_ticks: i32, current_tick: i32, combat_style_hrid: &str) {
        let tick_dmg = CombatUtilities::calculate_tick_value(damage as i64, total_ticks, current_tick);
        let clamped = tick_dmg.min(self.units[target].combat_details.current_hitpoints);
        self.units[target].combat_details.current_hitpoints -= clamped;

        let src_hrid = self.units[source_ref].hrid.clone();
        let tgt_unit = self.units[target].clone();
        self.sim_result.add_attack(&src_hrid, &tgt_unit.hrid, "damageOverTime", &clamped.to_string());
        if clamped > 0 {
            if self.units[source_ref].is_player {
                self.sim_result.add_player_damage_dealt(&src_hrid, clamped);
                self.sim_result.add_player_damage_dealt_by_ability(&src_hrid, "damageOverTime", clamped);

                // damage_taken debuff attribution
                let total_dt = self.units[target].combat_details.combat_stats.damage_taken;
                if total_dt > 0.0 {
                    let debuff_total = (clamped as f64 * total_dt / (1.0 + total_dt)).round() as i64;
                    if debuff_total > 0 {
                        for (unique, buff) in &self.units[target].combat_buffs.clone() {
                            if buff.type_hrid == "/buff_types/damage_taken" && buff.flat_boost > 0.0 {
                                let share = (debuff_total as f64 * buff.flat_boost / total_dt).round() as i64;
                                self.sim_result.add_debuff_damage(&src_hrid, unique, share);
                            }
                        }
                    }
                }

                // Resistance debuff attribution for DoT ticks.
                // Map combat_style -> the resistance buff_type that governs this damage.
                let res_buff_type = match combat_style_hrid {
                    "/combat_styles/slash" | "/combat_styles/stab" | "/combat_styles/smash"
                        => "/buff_types/armor",
                    "/combat_styles/water"  => "/buff_types/water_resistance",
                    "/combat_styles/magic"  => "/buff_types/nature_resistance",
                    "/combat_styles/fire"   => "/buff_types/fire_resistance",
                    "/combat_styles/ranged" => "/buff_types/armor",
                    _ => "",
                };
                if !res_buff_type.is_empty() && clamped > 0 {
                    for (unique, buff) in &self.units[target].combat_buffs.clone() {
                        if buff.type_hrid == res_buff_type && buff.flat_boost < 0.0 {
                            // Counterfactual: what was the total_resistance for this target?
                            // We don't have penetration here, so approximate:
                            // contribution ≈ clamped * |debuff| / (base_res + |debuff|)
                            // where base_res is inferred as current_res + |debuff|
                            let debuff_abs = buff.flat_boost.abs();
                            // Get current total resistance for this damage type
                            let cur_res = match res_buff_type {
                                "/buff_types/armor"              => self.units[target].combat_details.total_armor,
                                "/buff_types/water_resistance"   => self.units[target].combat_details.total_water_resistance,
                                "/buff_types/nature_resistance"  => self.units[target].combat_details.total_nature_resistance,
                                "/buff_types/fire_resistance"    => self.units[target].combat_details.total_fire_resistance,
                                _ => 0.0,
                            };
                            let res_without = cur_res + debuff_abs;
                            let ratio_actual  = 100.0 / (100.0 + cur_res.max(0.0));
                            let ratio_without = 100.0 / (100.0 + res_without.max(0.0));
                            let contribution = (clamped as f64 * (1.0 - ratio_without / ratio_actual)).round() as i64;
                            if contribution > 0 {
                                self.sim_result.add_debuff_damage(&src_hrid, unique, contribution);
                            }
                        }
                    }
                }
            }
            if self.units[target].is_player {
                self.sim_result.add_player_damage_taken(&tgt_unit.hrid, clamped);
                self.sim_result.add_player_damage_taken_by_source(&tgt_unit.hrid, &src_hrid, clamped);
                self.sim_result.add_player_damage_taken_by_ability(&tgt_unit.hrid, "damageOverTime", clamped);
            }
        }

        let is_dungeon = self.zone.as_ref().map(|z| z.is_dungeon).unwrap_or(false);
        if is_dungeon && self.units[target].is_player {
            let log = self.build_combat_log("", "damageOverTime", target, clamped);
            self.add_to_wipe_logs(log);
        }

        if current_tick < total_ticks {
            self.event_queue.add_event(CombatEvent {
                time: self.simulation_time + DOT_TICK_INTERVAL,
                kind: EventKind::DamageOverTime {
                    source_ref, target, damage, total_ticks, current_tick: current_tick + 1,
                    combat_style_hrid: combat_style_hrid.to_string(),
                },
            });
        }
        if self.units[target].combat_details.current_hitpoints == 0 {
            self.on_unit_died(target);
        }
        self.check_encounter_end();
    }

    fn process_regen_tick(&mut self) {
        for i in 0..self.num_players {
            if self.units[i].combat_details.current_hitpoints <= 0 { continue; }
            let hp_regen = (self.units[i].combat_details.max_hitpoints as f64
                * self.units[i].combat_details.combat_stats.hp_regen_per10).floor() as i64;
            let added_hp = self.units[i].add_hitpoints(hp_regen);
            let unit = self.units[i].clone();
            self.sim_result.add_hitpoints_gained(&unit, "regen", added_hp);

            let mp_regen = (self.units[i].combat_details.max_manapoints as f64
                * self.units[i].combat_details.combat_stats.mp_regen_per10).floor() as i64;
            let added_mp = self.units[i].add_manapoints(mp_regen);
            let unit = self.units[i].clone();
            self.sim_result.add_manapoints_gained(&unit, "regen", added_mp);

            if self.units[i].is_out_of_mana {
                self.event_queue.add_event(CombatEvent {
                    time: self.simulation_time,
                    kind: EventKind::AwaitCooldown { source: i },
                });
            }
        }
        self.event_queue.add_event(CombatEvent {
            time: self.simulation_time + REGEN_TICK_INTERVAL,
            kind: EventKind::RegenTick,
        });
    }

    fn process_enrage_tick(&mut self, encounter_time: i64) {
        let ei = match self.enemy_indices.clone() { Some(e) => e, None => return };
        let max_enrage_stack = 10;

        for &i in &ei {
            if self.units[i].combat_details.current_hitpoints <= 0 { continue; }
            let enrage_time = self.units[i].enrage_time;
            if enrage_time <= 0 { continue; }
            let now_stack = (encounter_time / enrage_time).min(max_enrage_stack) as i32;
            if now_stack <= 0 { continue; }

            let dmg_buff = Buff::inline("/buff_uniques/enrage_damage", "/buff_types/damage",
                now_stack as f64 * 0.1, 0.0, ENRAGE_TICK_INTERVAL);
            let acc_buff = Buff::inline("/buff_uniques/enrage_accuracy", "/buff_types/accuracy",
                now_stack as f64 * 0.1, 0.0, ENRAGE_TICK_INTERVAL);
            let t = self.simulation_time;
            self.units[i].add_buffs(vec![dmg_buff, acc_buff], t, i);
            self.sim_result.max_enrage_stack = self.sim_result.max_enrage_stack.max(now_stack);
        }

        self.event_queue.add_event(CombatEvent {
            time: self.simulation_time + ENRAGE_TICK_INTERVAL,
            kind: EventKind::EnrageTick { encounter_time: encounter_time + ENRAGE_TICK_INTERVAL },
        });
    }

    // -- Triggers check (food/drink) -------------------------------------------

    fn check_triggers(&mut self) {
        loop {
            let mut triggered = false;

            let player_indices: Vec<_> = (0..self.num_players)
                .filter(|&i| self.units[i].combat_details.current_hitpoints > 0)
                .collect();
            for &i in &player_indices {
                if self.check_triggers_for_unit(i) { triggered = true; }
            }

            let enemy_indices = self.enemy_indices.clone().unwrap_or_default();
            let alive_enemies: Vec<_> = enemy_indices.iter().copied()
                .filter(|&i| self.units[i].combat_details.current_hitpoints > 0).collect();
            for &i in &alive_enemies {
                if self.check_triggers_for_unit(i) { triggered = true; }
            }

            if !triggered { break; }
        }
    }

    fn check_triggers_for_unit(&mut self, unit_idx: UnitIdx) -> bool {
        let mut triggered = false;
        let is_player = self.units[unit_idx].is_player;

        let (friendlies, enemies_opt): (Vec<UnitIdx>, Option<Vec<UnitIdx>>) = if is_player {
            ((0..self.num_players).collect(), self.enemy_indices.clone())
        } else {
            (self.enemy_indices.clone().unwrap_or_default(), Some((0..self.num_players).collect()))
        };

        let target_idx = enemies_opt.as_ref().and_then(|e| {
            e.iter().find(|&&i| self.units[i].combat_details.current_hitpoints > 0).copied()
        });

        // Check food
        for food_slot in 0..3 {
            let should = self.units[unit_idx].food[food_slot].as_ref().map(|food| {
                // Check stun
                if self.units[unit_idx].is_stunned { return false; }
                let haste = self.units[unit_idx].combat_details.combat_stats.food_haste;
                let mut cd = food.cooldown_duration as f64;
                if haste > 0.0 { cd /= 1.0 + haste; }
                if food.last_used.saturating_add(cd as i64) > self.simulation_time { return false; }
                if food.triggers.is_empty() { return true; }
                food.triggers.iter().all(|t| {
                    let is_single = *crate::combatsimulator::data::combat_trigger_dependency_map()
                        .get(&t.dependency_hrid).unwrap_or(&false);
                    if is_single {
                        self.eval_trigger_single(t, unit_idx, target_idx)
                    } else {
                        self.eval_trigger_multi(t, &friendlies, &enemies_opt.clone().unwrap_or_default())
                    }
                })
            }).unwrap_or(false);
            if should {
                self.try_use_consumable(unit_idx, true, food_slot);
                triggered = true;
            }
        }

        // Check drinks
        for drink_slot in 0..3 {
            let should = self.units[unit_idx].drinks[drink_slot].as_ref().map(|drink| {
                if self.units[unit_idx].is_stunned { return false; }
                let conc = self.units[unit_idx].combat_details.combat_stats.drink_concentration;
                let mut cd = drink.cooldown_duration as f64;
                if conc > 0.0 { cd /= 1.0 + conc; }
                if drink.last_used.saturating_add(cd as i64) > self.simulation_time { return false; }
                if drink.triggers.is_empty() { return true; }
                drink.triggers.iter().all(|t| {
                    let is_single = *crate::combatsimulator::data::combat_trigger_dependency_map()
                        .get(&t.dependency_hrid).unwrap_or(&false);
                    if is_single {
                        self.eval_trigger_single(t, unit_idx, target_idx)
                    } else {
                        self.eval_trigger_multi(t, &friendlies, &enemies_opt.clone().unwrap_or_default())
                    }
                })
            }).unwrap_or(false);
            if should {
                self.try_use_consumable(unit_idx, false, drink_slot);
                triggered = true;
            }
        }

        triggered
    }

    fn try_use_consumable(&mut self, source_idx: UnitIdx, is_food: bool, slot: usize) -> bool {
        if self.units[source_idx].combat_details.current_hitpoints <= 0 { return false; }

        let consumable = {
            let slot_ref = if is_food { &self.units[source_idx].food[slot] } else { &self.units[source_idx].drinks[slot] };
            match slot_ref { Some(c) => c.clone(), None => return false }
        };

        let sim_time = self.simulation_time;
        if is_food {
            if let Some(ref mut f) = self.units[source_idx].food[slot] { f.last_used = sim_time; }
        } else {
            if let Some(ref mut d) = self.units[source_idx].drinks[slot] { d.last_used = sim_time; }
        }

        let conc = self.units[source_idx].combat_details.combat_stats.drink_concentration;
        let haste = self.units[source_idx].combat_details.combat_stats.food_haste;
        let mut cd = consumable.cooldown_duration as f64;
        if consumable.is_drink() && conc > 0.0 { cd /= 1.0 + conc; }
        else if consumable.is_food() && haste > 0.0 { cd /= 1.0 + haste; }

        self.event_queue.add_event(CombatEvent {
            time: self.simulation_time + cd as i64,
            kind: EventKind::CooldownReady,
        });

        let unit = self.units[source_idx].clone();
        self.sim_result.add_consumable_use(&unit, &consumable.hrid);

        if consumable.recovery_duration == 0 {
            if consumable.hitpoint_restore > 0 {
                let added = self.units[source_idx].add_hitpoints(consumable.hitpoint_restore);
                let unit = self.units[source_idx].clone();
                self.sim_result.add_hitpoints_gained(&unit, &consumable.hrid, added);
            }
            if consumable.manapoint_restore > 0 {
                let added = self.units[source_idx].add_manapoints(consumable.manapoint_restore);
                let unit = self.units[source_idx].clone();
                self.sim_result.add_manapoints_gained(&unit, &consumable.hrid, added);
                if self.units[source_idx].is_out_of_mana {
                    self.event_queue.add_event(CombatEvent {
                        time: self.simulation_time,
                        kind: EventKind::AwaitCooldown { source: source_idx },
                    });
                }
            }
        } else {
            let total_ticks = (consumable.recovery_duration / HOT_TICK_INTERVAL) as i32;
            self.event_queue.add_event(CombatEvent {
                time: self.simulation_time + HOT_TICK_INTERVAL,
                kind: EventKind::ConsumableTick {
                    source: source_idx, consumable_hrid: consumable.hrid.clone(),
                    total_ticks, current_tick: 1,
                },
            });
        }

        // Apply buffs
        for buff in &consumable.buffs {
            let mut cur = buff.clone();
            if consumable.is_drink() && conc > 0.0 {
                cur.ratio_boost *= 1.0 + conc;
                cur.flat_boost *= 1.0 + conc;
                cur.duration = (cur.duration as f64 / (1.0 + conc)) as i64;
            }
            let t = self.simulation_time;
            self.units[source_idx].add_buff(cur.clone(), t, source_idx);
            let dur = cur.duration;
            self.event_queue.add_event(CombatEvent {
                time: self.simulation_time + dur,
                kind: EventKind::CheckBuffExpiration { source: source_idx },
            });
        }
        true
    }

    // -- Status effect helpers -------------------------------------------------

    fn apply_curse(&mut self, source_idx: UnitIdx, target_idx: UnitIdx) {
        let curse_expire = 15_000_000_000i64;
        let current_amount = self.event_queue.get_matching(|e|
            matches!(&e.kind, EventKind::CurseExpiration { source, .. } if *source == target_idx)
        ).and_then(|e| if let EventKind::CurseExpiration { curse_amount, .. } = e.kind { Some(curse_amount) } else { None })
        .unwrap_or(0);
        let at = target_idx;
        self.event_queue.clear_matching(|e| matches!(&e.kind, EventKind::CurseExpiration { source, .. } if *source == at));

        let new_amount = (current_amount + 1).min(5);
        let curse_stat = self.units[source_idx].combat_details.combat_stats.curse;
        let buff = Buff::inline("/buff_uniques/curse", "/buff_types/damage_taken",
            0.0, curse_stat * new_amount as f64, curse_expire);
        let t = self.simulation_time;
        self.units[target_idx].add_buff(buff, t, source_idx);
        self.event_queue.add_event(CombatEvent {
            time: self.simulation_time + curse_expire,
            kind: EventKind::CurseExpiration { source: target_idx, curse_amount: new_amount },
        });
    }

    fn apply_fury(&mut self, source_idx: UnitIdx, did_hit: bool) {
        let fury_expire = 15_000_000_000i64;
        let max_stack = 5;
        let current_amount = self.event_queue.get_matching(|e|
            matches!(&e.kind, EventKind::FuryExpiration { source, .. } if *source == source_idx)
        ).and_then(|e| if let EventKind::FuryExpiration { fury_amount, .. } = e.kind { Some(fury_amount) } else { None })
        .unwrap_or(0.0);
        let si = source_idx;
        self.event_queue.clear_matching(|e| matches!(&e.kind, EventKind::FuryExpiration { source, .. } if *source == si));

        let new_amount = if did_hit { (current_amount + 1.0).min(max_stack as f64) } else { current_amount / 2.0 };
        let fury_stat = self.units[source_idx].combat_details.combat_stats.fury;

        if new_amount > 0.0 {
            let acc_buff = Buff::inline("/buff_uniques/fury_accuracy", "/buff_types/fury_accuracy",
                new_amount * fury_stat, 0.0, fury_expire);
            let dmg_buff = Buff::inline("/buff_uniques/fury_damage", "/buff_types/fury_damage",
                new_amount * fury_stat, 0.0, fury_expire);
            let t = self.simulation_time;
            self.units[source_idx].add_buffs(vec![acc_buff, dmg_buff], t, source_idx);
            self.event_queue.add_event(CombatEvent {
                time: self.simulation_time + fury_expire,
                kind: EventKind::FuryExpiration { source: source_idx, fury_amount: new_amount },
            });
        } else {
            self.units[source_idx].remove_buff_by_unique("/buff_uniques/fury_accuracy", source_idx);
            self.units[source_idx].remove_buff_by_unique("/buff_uniques/fury_damage", source_idx);
        }
    }

    fn apply_weaken(&mut self, source_idx: UnitIdx, target_idx: UnitIdx) {
        let weaken_expire = 15_000_000_000i64;
        let current_amount = self.event_queue.get_matching(|e|
            matches!(&e.kind, EventKind::WeakenExpiration { source, .. } if *source == source_idx)
        ).and_then(|e| if let EventKind::WeakenExpiration { weaken_amount, .. } = e.kind { Some(weaken_amount) } else { None })
        .unwrap_or(0);
        let si = source_idx;
        self.event_queue.clear_matching(|e| matches!(&e.kind, EventKind::WeakenExpiration { source, .. } if *source == si));

        let new_amount = (current_amount + 1).min(5);
        let weaken_stat = self.units[target_idx].combat_details.combat_stats.weaken;
        let buff = Buff::inline("/buff_uniques/weaken", "/buff_types/damage",
            -1.0 * weaken_stat * new_amount as f64, 0.0, weaken_expire);
        let t = self.simulation_time;
        self.units[source_idx].add_buff(buff, t, target_idx);
        self.event_queue.add_event(CombatEvent {
            time: self.simulation_time + weaken_expire,
            kind: EventKind::WeakenExpiration { source: source_idx, weaken_amount: new_amount },
        });
    }
}

fn compare_trigger(comparator: &str, val: f64, threshold: f64) -> bool {
    match comparator {
        "/combat_trigger_comparators/greater_than_equal" => val >= threshold,
        "/combat_trigger_comparators/less_than_equal"    => val <= threshold,
        "/combat_trigger_comparators/is_active"          => val != 0.0,
        "/combat_trigger_comparators/is_inactive"        => val == 0.0,
        _ => false,
    }
}