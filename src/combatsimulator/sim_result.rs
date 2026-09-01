use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::combatsimulator::{combat_unit::CombatUnit, data};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TimeSpentAliveEntry {
    pub name: String,
    pub time_spent_alive: i64,
    pub spawned_at: i64,
    pub alive: bool,
    pub count: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ManaTimeEntry {
    pub is_out_of_mana: bool,
    pub start_time_for_out_of_mana: i64,
    pub total_time_for_out_of_mana: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct WipeEvent {
    pub simulation_time: i64,
    pub logs: Vec<serde_json::Value>,
    pub wave: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TimeSeriesPlayer {
    pub hp: Vec<i64>,
    pub mp: Vec<i64>,
    pub max_hp: Vec<i64>,
    pub max_mp: Vec<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TimeSeriesData {
    pub timestamps: Vec<i64>,
    pub players: HashMap<String, TimeSeriesPlayer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimResult {
    pub deaths: HashMap<String, i32>,
    pub player_deaths: HashMap<String, i32>,
    pub experience_gained: HashMap<String, HashMap<String, f64>>,
    pub encounters: i32,
    pub attacks: HashMap<String, HashMap<String, HashMap<String, HashMap<String, i32>>>>,
    pub consumables_used: HashMap<String, HashMap<String, i32>>,
    pub hitpoints_gained: HashMap<String, HashMap<String, i64>>,
    pub manapoints_gained: HashMap<String, HashMap<String, i64>>,
    pub debuff_on_level_gap: HashMap<String, f64>,
    pub drop_rate_multiplier: HashMap<String, f64>,
    pub rare_find_multiplier: HashMap<String, f64>,
    pub combat_drop_quantity: HashMap<String, f64>,
    // Expected loot sell value per player (coins from all sellable drops)
    pub loot_value: HashMap<String, f64>,
    // Expected chest counts per chest type (summed across all players)
    pub chest_count: HashMap<String, f64>,
    // Damage attributed to specific damage_taken debuffs (curse, fracturing_impact, etc.)
    // outer key = player hrid, inner key = debuff unique_hrid
    pub debuff_damage_dealt: HashMap<String, HashMap<String, i64>>,
    pub player_ran_out_of_mana: HashMap<String, bool>,
    pub player_ran_out_of_mana_time: HashMap<String, ManaTimeEntry>,
    pub mana_used: HashMap<String, HashMap<String, i64>>,
    pub time_spent_alive: Vec<TimeSpentAliveEntry>,
    pub boss_spawns: Vec<String>,
    pub hitpoints_spent: HashMap<String, HashMap<String, i64>>,
    pub player_damage_dealt: HashMap<String, i64>,
    pub player_damage_taken: HashMap<String, i64>,
    pub player_damage_taken_by_source: HashMap<String, HashMap<String, i64>>,
    pub player_damage_taken_by_ability: HashMap<String, HashMap<String, i64>>,
    pub player_damage_dealt_by_ability: HashMap<String, HashMap<String, i64>>,
    pub zone_name: Option<String>,
    pub difficulty_tier: Option<i32>,
    pub labyrinth_name: Option<String>,
    pub room_level: Option<i32>,
    pub is_dungeon: bool,
    pub is_labyrinth: bool,
    pub dungeons_completed: i32,
    pub dungeons_failed: i32,
    pub max_wave_reached: i32,
    pub number_of_players: usize,
    pub max_enrage_stack: i32,
    pub min_dungeon_time: i64,
    pub max_dungeon_time: i64,
    pub last_dungeon_finish_time: i64,
    pub last_encounter_finish_time: i64,
    pub laby_attempt_count: i32,
    pub wipe_events: Vec<WipeEvent>,
    pub simulated_time: i64,
    pub time_series_data: TimeSeriesData,
    // Set only when the simulator was asked to stop after the first dungeon
    // attempt resolves (see CombatSimulator::set_stop_after_dungeon_result):
    // Some(true) = the attempt was cleared, Some(false) = the party wiped,
    // None = the attempt didn't resolve within the given time limit.
    pub dungeon_attempt_won: Option<bool>,
    // Combined % of enemy hitpoints (current / max, summed across all enemies
    // still alive in the encounter) remaining at the moment the party wiped.
    // Only set on a wipe; None otherwise.
    pub enemy_hp_pct_remaining: Option<f64>,
}

impl SimResult {
    pub fn new(
        zone_hrid: Option<&str>,
        zone_difficulty_tier: Option<i32>,
        labyrinth_hrid: Option<&str>,
        room_level: Option<i32>,
        number_of_players: usize,
        is_labyrinth: bool,
    ) -> Self {
        let mut player_ran_out_of_mana = HashMap::new();
        for i in 1..=5 {
            player_ran_out_of_mana.insert(format!("player{}", i), false);
        }

        SimResult {
            deaths: HashMap::new(),
            player_deaths: HashMap::new(),
            experience_gained: HashMap::new(),
            encounters: 0,
            attacks: HashMap::new(),
            consumables_used: HashMap::new(),
            hitpoints_gained: HashMap::new(),
            manapoints_gained: HashMap::new(),
            debuff_on_level_gap: HashMap::new(),
            drop_rate_multiplier: HashMap::new(),
            rare_find_multiplier: HashMap::new(),
            combat_drop_quantity: HashMap::new(),
            loot_value: HashMap::new(),
            chest_count: HashMap::new(),
            debuff_damage_dealt: HashMap::new(),
            player_ran_out_of_mana,
            player_ran_out_of_mana_time: HashMap::new(),
            mana_used: HashMap::new(),
            time_spent_alive: Vec::new(),
            boss_spawns: Vec::new(),
            hitpoints_spent: HashMap::new(),
            player_damage_dealt: HashMap::new(),
            player_damage_taken: HashMap::new(),
            player_damage_taken_by_source: HashMap::new(),
            player_damage_taken_by_ability: HashMap::new(),
            player_damage_dealt_by_ability: HashMap::new(),
            zone_name: zone_hrid.map(String::from),
            difficulty_tier: zone_difficulty_tier,
            labyrinth_name: labyrinth_hrid.map(String::from),
            room_level,
            is_dungeon: false,
            is_labyrinth,
            dungeons_completed: 0,
            dungeons_failed: 0,
            max_wave_reached: 0,
            number_of_players,
            max_enrage_stack: 0,
            min_dungeon_time: 0,
            max_dungeon_time: 0,
            last_dungeon_finish_time: 0,
            last_encounter_finish_time: 0,
            laby_attempt_count: 0,
            wipe_events: Vec::new(),
            simulated_time: 0,
            time_series_data: TimeSeriesData::default(),
            dungeon_attempt_won: None,
            enemy_hp_pct_remaining: None,
        }
    }

    pub fn add_death(&mut self, unit: &CombatUnit) {
        *self.deaths.entry(unit.hrid.clone()).or_insert(0) += 1;
        if unit.is_player {
            *self.player_deaths.entry(unit.hrid.clone()).or_insert(0) += 1;
        }
    }

    pub fn update_time_spent_alive(&mut self, name: &str, alive: bool, time: i64) {
        if let Some(entry) = self.time_spent_alive.iter_mut().find(|e| e.name == name) {
            if alive {
                entry.alive = true;
                entry.spawned_at = time;
            } else {
                let time_alive = time - entry.spawned_at;
                entry.alive = false;
                entry.time_spent_alive += time_alive;
                entry.count += 1;
            }
        } else if alive {
            self.time_spent_alive.push(TimeSpentAliveEntry {
                name: name.to_string(),
                time_spent_alive: 0,
                spawned_at: time,
                alive: true,
                count: 0,
            });
        }
    }

    pub fn update_dungeon_finish(&mut self, begin_flag: &str, finish_time: i64) {
        if let Some(entry) = self.time_spent_alive.iter().find(|e| e.name == begin_flag) {
            let current_time = finish_time - entry.spawned_at;
            if self.min_dungeon_time == 0 || self.min_dungeon_time > current_time {
                self.min_dungeon_time = current_time;
            }
            if self.max_dungeon_time < current_time {
                self.max_dungeon_time = current_time;
            }
        }
    }

    pub fn add_experience_gain(&mut self, unit: &CombatUnit, experience: f64) {
        if !unit.is_player { return; }

        let exp_map = self.experience_gained.entry(unit.hrid.clone()).or_insert_with(|| {
            let mut m = HashMap::new();
            for s in &["stamina","intelligence","attack","melee","defense","ranged","magic"] {
                m.insert(s.to_string(), 0.0);
            }
            m
        });

        let mut rates: HashMap<&str, f64> = HashMap::new();
        let primary = &unit.combat_details.combat_stats.primary_training;
        // primary_training looks like "/skills/melee"
        let primary_skill = primary.split('/').last().unwrap_or("");
        if !primary_skill.is_empty() {
            *rates.entry(primary_skill).or_insert(0.0) += 0.3;
        }

        let style_map = data::combat_style_detail_map();
        let style = &unit.combat_details.combat_stats.combat_style_hrid;
        let skill_exp_map = style_map.get(style)
            .and_then(|s| s["skillExpMap"].as_object().cloned())
            .unwrap_or_default();
        let skill_count = skill_exp_map.len() as f64;

        let focus = &unit.combat_details.combat_stats.focus_training;
        let focus_skill = focus.split('/').last().unwrap_or("");

        if !focus.is_empty() && skill_exp_map.contains_key(focus.as_str()) {
            *rates.entry(focus_skill).or_insert(0.0) += 0.7;
        } else if skill_count > 0.0 {
            for skill_hrid in skill_exp_map.keys() {
                let skill = skill_hrid.split('/').last().unwrap_or("");
                *rates.entry(skill).or_insert(0.0) += 0.7 / skill_count;
            }
        }

        let combat_exp_mult = 1.0 + unit.combat_details.combat_stats.combat_experience;
        let debuff = 1.0 + unit.debuff_on_level_gap;

        for (skill_key, rate) in &rates {
            if *rate <= 0.0 { continue; }
            let skill_exp_bonus_key = format!("{}_experience", skill_key);
            let skill_exp_bonus = get_skill_experience(&unit.combat_details.combat_stats, &skill_exp_bonus_key);
            let skill_exp = rate * (1.0 + skill_exp_bonus);
            let gained = experience * combat_exp_mult * skill_exp * debuff;
            if let Some(v) = exp_map.get_mut(*skill_key) {
                *v += gained;
            }
        }
    }

    pub fn add_encounter_end(&mut self) {
        self.encounters += 1;
    }

    pub fn add_attack(&mut self, source: &str, target: &str, ability: &str, damage: &str) {
        *self.attacks
            .entry(source.to_string()).or_default()
            .entry(target.to_string()).or_default()
            .entry(ability.to_string()).or_default()
            .entry(damage.to_string()).or_insert(0) += 1;
    }

    pub fn add_consumable_use(&mut self, unit: &CombatUnit, consumable_hrid: &str) {
        *self.consumables_used
            .entry(unit.hrid.clone()).or_default()
            .entry(consumable_hrid.to_string()).or_insert(0) += 1;
    }

    pub fn add_hitpoints_gained(&mut self, unit: &CombatUnit, source: &str, amount: i64) {
        *self.hitpoints_gained
            .entry(unit.hrid.clone()).or_default()
            .entry(source.to_string()).or_insert(0) += amount;
    }

    pub fn add_manapoints_gained(&mut self, unit: &CombatUnit, source: &str, amount: i64) {
        *self.manapoints_gained
            .entry(unit.hrid.clone()).or_default()
            .entry(source.to_string()).or_insert(0) += amount;
    }

    pub fn add_hitpoints_spent(&mut self, unit: &CombatUnit, source: &str, amount: i64) {
        *self.hitpoints_spent
            .entry(unit.hrid.clone()).or_default()
            .entry(source.to_string()).or_insert(0) += amount;
    }

    pub fn add_player_damage_dealt(&mut self, player_hrid: &str, amount: i64) {
        *self.player_damage_dealt.entry(player_hrid.to_string()).or_insert(0) += amount;
    }

    /// Record damage attributable to a specific damage_taken debuff (by its unique_hrid).
    pub fn add_debuff_damage(&mut self, player_hrid: &str, debuff_unique: &str, amount: i64) {
        if amount > 0 {
            *self.debuff_damage_dealt
                .entry(player_hrid.to_string()).or_default()
                .entry(debuff_unique.to_string()).or_insert(0) += amount;
        }
    }

    pub fn add_player_damage_taken(&mut self, player_hrid: &str, amount: i64) {
        *self.player_damage_taken.entry(player_hrid.to_string()).or_insert(0) += amount;
    }

    pub fn add_player_damage_taken_by_source(&mut self, player_hrid: &str, source_hrid: &str, amount: i64) {
        *self.player_damage_taken_by_source
            .entry(player_hrid.to_string()).or_default()
            .entry(source_hrid.to_string()).or_insert(0) += amount;
    }

    pub fn add_player_damage_taken_by_ability(&mut self, player_hrid: &str, ability: &str, amount: i64) {
        *self.player_damage_taken_by_ability
            .entry(player_hrid.to_string()).or_default()
            .entry(ability.to_string()).or_insert(0) += amount;
    }

    pub fn add_player_damage_dealt_by_ability(&mut self, player_hrid: &str, ability: &str, amount: i64) {
        *self.player_damage_dealt_by_ability
            .entry(player_hrid.to_string()).or_default()
            .entry(ability.to_string()).or_insert(0) += amount;
    }

    pub fn set_drop_rate_multipliers(&mut self, unit: &CombatUnit) {
        self.drop_rate_multiplier.insert(unit.hrid.clone(), 1.0 + unit.combat_details.combat_stats.combat_drop_rate);
        self.rare_find_multiplier.insert(unit.hrid.clone(), 1.0 + unit.combat_details.combat_stats.combat_rare_find);
        self.combat_drop_quantity.insert(unit.hrid.clone(), unit.combat_details.combat_stats.combat_drop_quantity);
        self.debuff_on_level_gap.insert(unit.hrid.clone(), unit.debuff_on_level_gap);
    }

    /// Compute expected loot sell value from deaths × per-player drop multipliers.
    /// Called once after the simulation ends.
    pub fn compute_loot_value(&mut self, market_prices: Option<&std::collections::HashMap<String, f64>>) {
        use serde_json::Value;
        let monsters = crate::combatsimulator::data::combat_monster_detail_map();
        let items    = crate::combatsimulator::data::item_detail_map();
        let empty_arr: Vec<Value> = vec![];

        let player_hrids: Vec<String> = self.drop_rate_multiplier.keys().cloned().collect();
        let mut loot:   HashMap<String, f64> = HashMap::new();
        let mut chests: HashMap<String, f64> = HashMap::new();

        for (monster_hrid, &kill_count) in &self.deaths {
            let m = match monsters.get(monster_hrid) { Some(m) => m, None => continue };
            let kill_f = kill_count as f64;

            for player_hrid in &player_hrids {
                let drm = *self.drop_rate_multiplier.get(player_hrid).unwrap_or(&1.0);
                let rfm = *self.rare_find_multiplier .get(player_hrid).unwrap_or(&1.0);
                let cdq = *self.combat_drop_quantity  .get(player_hrid).unwrap_or(&0.0);

                let tier_f = self.difficulty_tier.unwrap_or(0) as f64;

                for entry in m["dropTable"].as_array().unwrap_or(&empty_arr) {
                    let ihrid = entry["itemHrid"].as_str().unwrap_or("");
                    let base_rate = entry["dropRate"].as_f64().unwrap_or(0.0);
                    let tier_bonus = entry["dropRatePerDifficultyTier"].as_f64().unwrap_or(0.0) * tier_f;
                    let effective_rate = (base_rate + tier_bonus).max(0.0);
                    let rate  = (effective_rate * drm).min(1.0);
                    let avg_q = (entry["minCount"].as_f64().unwrap_or(1.0)
                               + entry["maxCount"].as_f64().unwrap_or(1.0)) / 2.0 + cdq;
                    // Coins have sellPrice=0 in the data but are worth 1 each by definition.
                    // Use live market price (lowestAsk) if available, else static sellPrice.
                    let sell = if ihrid == "/items/coin" {
                        1.0
                    } else if let Some(mp) = market_prices {
                        mp.get(ihrid).copied()
                            .unwrap_or_else(|| items.get(ihrid).and_then(|i| i["sellPrice"].as_f64()).unwrap_or(0.0))
                    } else {
                        items.get(ihrid).and_then(|i| i["sellPrice"].as_f64()).unwrap_or(0.0)
                    };
                    if sell > 0.0 {
                        *loot.entry(player_hrid.clone()).or_insert(0.0) += kill_f * rate * avg_q * sell;
                    }
                }

                for entry in m["rareDropTable"].as_array().unwrap_or(&empty_arr) {
                    let ihrid = entry["itemHrid"].as_str().unwrap_or("");
                    let base_rate = entry["dropRate"].as_f64().unwrap_or(0.0);
                    let tier_bonus = entry["dropRatePerDifficultyTier"].as_f64().unwrap_or(0.0) * tier_f;
                    let effective_rate = (base_rate + tier_bonus).max(0.0);
                    let rate  = (effective_rate * drm * rfm).min(1.0);
                    let avg_q = (entry["minCount"].as_f64().unwrap_or(1.0)
                               + entry["maxCount"].as_f64().unwrap_or(1.0)) / 2.0;
                    let item  = match items.get(ihrid) { Some(i) => i, None => continue };
                    if item["isOpenable"].as_bool().unwrap_or(false) {
                        *chests.entry(ihrid.to_string()).or_insert(0.0) += kill_f * rate * avg_q;
                    } else {
                        let sell = if let Some(mp) = market_prices {
                            mp.get(ihrid).copied()
                                .unwrap_or_else(|| item["sellPrice"].as_f64().unwrap_or(0.0))
                        } else {
                            item["sellPrice"].as_f64().unwrap_or(0.0)
                        };
                        if sell > 0.0 {
                            *loot.entry(player_hrid.clone()).or_insert(0.0) += kill_f * rate * avg_q * sell;
                        }
                    }
                }
            }
        }

        self.loot_value = loot;
        self.chest_count = chests;
    }

    pub fn set_mana_used(&mut self, unit: &CombatUnit) {
        let entry = self.mana_used.entry(unit.hrid.clone()).or_default();
        for (k, v) in &unit.ability_mana_costs {
            entry.insert(k.clone(), *v);
        }
    }

    pub fn add_ran_out_of_mana_count(&mut self, unit: &CombatUnit, is_oom: bool, time: i64) {
        if is_oom {
            self.player_ran_out_of_mana.insert(unit.hrid.clone(), true);
        }
        let entry = self.player_ran_out_of_mana_time.entry(unit.hrid.clone()).or_default();
        if is_oom {
            if !entry.is_out_of_mana {
                entry.is_out_of_mana = true;
                entry.start_time_for_out_of_mana = time;
            }
        } else {
            if entry.is_out_of_mana {
                entry.is_out_of_mana = false;
                entry.total_time_for_out_of_mana += time - entry.start_time_for_out_of_mana;
            }
        }
    }

    pub fn add_wipe_event(&mut self, logs: Vec<serde_json::Value>, simulation_time: i64, wave: i32) {
        self.wipe_events.push(WipeEvent { simulation_time, logs, wave });
    }

    pub fn add_time_series_snapshot(&mut self, time: i64, players: &[CombatUnit]) {
        self.time_series_data.timestamps.push(time);
        for player in players {
            let entry = self.time_series_data.players.entry(player.hrid.clone()).or_default();
            entry.hp.push(player.combat_details.current_hitpoints);
            entry.mp.push(player.combat_details.current_manapoints);
            entry.max_hp.push(player.combat_details.max_hitpoints);
            entry.max_mp.push(player.combat_details.max_manapoints);
        }
    }
}

fn get_skill_experience(stats: &crate::combatsimulator::combat_unit::CombatStats, key: &str) -> f64 {
    match key {
        "stamina_experience" => stats.stamina_experience,
        "intelligence_experience" => stats.intelligence_experience,
        "attack_experience" => stats.attack_experience,
        "defense_experience" => stats.defense_experience,
        "melee_experience" => stats.melee_experience,
        "ranged_experience" => stats.ranged_experience,
        "magic_experience" => stats.magic_experience,
        _ => 0.0,
    }
}