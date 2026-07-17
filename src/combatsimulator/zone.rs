use serde_json::Value;
use rand::Rng;
use crate::combatsimulator::{buff::Buff, combat_unit::CombatUnit, data, monster::MonsterBuilder};

pub struct Zone {
    pub hrid: String,
    pub difficulty_tier: i32,
    pub monster_spawn_info: Value,
    pub dungeon_spawn_info: Value,
    pub encounters_killed: i32,
    pub buffs: Vec<Buff>,
    pub is_dungeon: bool,
    pub dungeons_completed: i32,
    pub dungeons_failed: i32,
    pub final_wave: bool,
}

impl Zone {
    pub fn new(hrid: String, difficulty_tier: i32) -> Self {
        let actions = data::action_detail_map();
        let game_zone = actions.get(&hrid)
            .unwrap_or_else(|| panic!("No zone found for hrid: {}", hrid));

        let monster_spawn_info = game_zone["combatZoneInfo"]["fightInfo"].clone();
        let mut dungeon_spawn_info = game_zone["combatZoneInfo"]["dungeonInfo"].clone();
        let is_dungeon = game_zone["combatZoneInfo"]["isDungeon"].as_bool().unwrap_or(false);

        let buffs = game_zone["buffs"].as_array()
            .map(|arr| arr.iter().map(|b| Buff::from_value(b, 1)).collect())
            .unwrap_or_default();

        Zone {
            hrid,
            difficulty_tier,
            monster_spawn_info,
            dungeon_spawn_info,
            encounters_killed: 1,
            buffs,
            is_dungeon,
            dungeons_completed: 0,
            dungeons_failed: 0,
            final_wave: false,
        }
    }

    pub fn get_random_encounter(&mut self, num_players: i32) -> Vec<CombatUnit> {
        let mut rng = rand::thread_rng();

        // Boss spawn check
        if let Some(boss_spawns) = self.monster_spawn_info["bossSpawns"].as_array() {
            let battles_per_boss = self.monster_spawn_info["battlesPerBoss"].as_i64().unwrap_or(10) as i32;
            if !boss_spawns.is_empty() && self.encounters_killed == battles_per_boss {
                self.encounters_killed = 1;
                return boss_spawns.iter()
                    .map(|m| {
                        let h = m["combatMonsterHrid"].as_str().unwrap_or("").to_string();
                        let tier = m["difficultyTier"].as_i64().unwrap_or(0) as i32 + self.difficulty_tier;
                        MonsterBuilder::new(h, tier, 0, num_players)
                    })
                    .collect();
            }
        }

        let spawn_info = &self.monster_spawn_info["randomSpawnInfo"];
        let spawns = spawn_info["spawns"].as_array().cloned().unwrap_or_default();
        let max_spawn = spawn_info["maxSpawnCount"].as_i64().unwrap_or(1) as usize;
        let max_strength = spawn_info["maxTotalStrength"].as_f64().unwrap_or(f64::MAX);
        let total_weight: f64 = spawns.iter().map(|s| s["rate"].as_f64().unwrap_or(0.0)).sum();

        let mut encounter = Vec::new();
        let mut total_strength = 0.0;

        'outer: for _ in 0..max_spawn {
            let mut random_weight = total_weight * rng.gen::<f64>();
            let mut cumulative = 0.0;
            for spawn in &spawns {
                cumulative += spawn["rate"].as_f64().unwrap_or(0.0);
                if random_weight <= cumulative {
                    let strength = spawn["strength"].as_f64().unwrap_or(0.0);
                    total_strength += strength;
                    if total_strength <= max_strength {
                        let h = spawn["combatMonsterHrid"].as_str().unwrap_or("").to_string();
                        let tier = spawn["difficultyTier"].as_i64().unwrap_or(0) as i32 + self.difficulty_tier;
                        encounter.push(MonsterBuilder::new(h, tier, 0, num_players));
                    } else {
                        break 'outer;
                    }
                    break;
                }
            }
        }

        self.encounters_killed += 1;
        encounter
    }

    pub fn get_next_wave(&mut self, num_players: i32) -> Vec<CombatUnit> {
        let mut rng = rand::thread_rng();

        let max_waves = self.dungeon_spawn_info["maxWaves"].as_i64().unwrap_or(0) as i32;
        if self.encounters_killed > max_waves {
            self.dungeons_completed += 1;
            self.encounters_killed = 1;
        }

        let wave_str = self.encounters_killed.to_string();

        // Fixed spawns check
        if let Some(fixed_map) = self.dungeon_spawn_info["fixedSpawnsMap"].as_object() {
            if let Some(monsters) = fixed_map.get(&wave_str) {
                let result: Vec<CombatUnit> = monsters.as_array().unwrap_or(&vec![])
                    .iter()
                    .map(|m| {
                        let h = m["combatMonsterHrid"].as_str().unwrap_or("").to_string();
                        let tier = m["difficultyTier"].as_i64().unwrap_or(0) as i32 + self.difficulty_tier;
                        MonsterBuilder::new(h, tier, 0, num_players)
                    })
                    .collect();
                self.encounters_killed += 1;
                return result;
            }
        }

        // Random spawns from the appropriate wave range
        let random_spawn_map = self.dungeon_spawn_info["randomSpawnInfoMap"].as_object()
            .cloned()
            .unwrap_or_default();
        let mut wave_keys: Vec<i32> = random_spawn_map.keys()
            .filter_map(|k| k.parse::<i32>().ok())
            .collect();
        wave_keys.sort();

        let spawn_info = if self.encounters_killed > *wave_keys.last().unwrap_or(&0) {
            let last_key = wave_keys.last().unwrap().to_string();
            self.dungeon_spawn_info["randomSpawnInfoMap"][&last_key].clone()
        } else {
            let mut chosen_key = wave_keys[0].to_string();
            for w in wave_keys.windows(2) {
                if self.encounters_killed >= w[0] && self.encounters_killed <= w[1] {
                    chosen_key = w[0].to_string();
                    break;
                }
            }
            self.dungeon_spawn_info["randomSpawnInfoMap"][&chosen_key].clone()
        };

        let spawns = spawn_info["spawns"].as_array().cloned().unwrap_or_default();
        let max_spawn = spawn_info["maxSpawnCount"].as_i64().unwrap_or(1) as usize;
        let max_strength = spawn_info["maxTotalStrength"].as_f64().unwrap_or(f64::MAX);
        let total_weight: f64 = spawns.iter().map(|s| s["rate"].as_f64().unwrap_or(0.0)).sum();

        let mut encounter = Vec::new();
        let mut total_strength = 0.0;

        'outer: for _ in 0..max_spawn {
            let mut cumulative = 0.0;
            let random_weight = total_weight * rng.gen::<f64>();
            for spawn in &spawns {
                cumulative += spawn["rate"].as_f64().unwrap_or(0.0);
                if random_weight <= cumulative {
                    let strength = spawn["strength"].as_f64().unwrap_or(0.0);
                    total_strength += strength;
                    if total_strength <= max_strength {
                        let h = spawn["combatMonsterHrid"].as_str().unwrap_or("").to_string();
                        let tier = spawn["difficultyTier"].as_i64().unwrap_or(0) as i32 + self.difficulty_tier;
                        encounter.push(MonsterBuilder::new(h, tier, 0, num_players));
                    } else {
                        break 'outer;
                    }
                    break;
                }
            }
        }

        self.encounters_killed += 1;
        encounter
    }

    pub fn fail_wave(&mut self) {
        self.dungeons_failed += 1;
        self.encounters_killed = 1;
    }
}
