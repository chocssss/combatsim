use serde_json::Value;
use crate::combatsimulator::{
    ability::Ability,
    combat_unit::CombatUnit,
    data,
    drops::Drops,
    player::set_combat_stat_by_name,
};

const LABYRINTH_MONSTER_BASE_ROOM_LEVEL: f64 = 100.0;

const MONSTER_STATS: &[&str] = &[
    "stabAccuracy","slashAccuracy","smashAccuracy","rangedAccuracy","magicAccuracy",
    "stabDamage","slashDamage","smashDamage","rangedDamage","magicDamage",
    "defensiveDamage","taskDamage",
    "physicalAmplify","waterAmplify","natureAmplify","fireAmplify","healingAmplify",
    "stabEvasion","slashEvasion","smashEvasion","rangedEvasion","magicEvasion",
    "armor","waterResistance","natureResistance","fireResistance",
    "maxHitpoints","maxManapoints","maxHitpointsRatio","maxManapointsRatio",
    "lifeSteal","hpRegenPer10","mpRegenPer10",
    "physicalThorns","elementalThorns",
    "combatDropRate","combatRareFind","combatDropQuantity","combatExperience",
    "criticalRate","criticalDamage",
    "armorPenetration","waterPenetration","naturePenetration","firePenetration",
    "abilityHaste","tenacity","manaLeech","castSpeed",
    "threat","parry","mayhem","pierce","curse","fury","weaken","ripple","bloom","blaze",
    "attackSpeed","foodHaste","drinkConcentration",
    "autoAttackDamage","abilityDamage","retaliation",
];

pub struct MonsterBuilder;

impl MonsterBuilder {
    pub fn new(hrid: String, difficulty_tier: i32, room_level_opt: i32, num_players: i32) -> CombatUnit {
        let mut unit = CombatUnit::new_base();
        unit.is_player = false;
        unit.hrid = hrid.clone();

        let room_level = if room_level_opt <= 0 {
            LABYRINTH_MONSTER_BASE_ROOM_LEVEL as i32
        } else {
            room_level_opt
        };
        // Store difficulty_tier + room_level in experience_rate temporarily (0 = not yet set)
        // We use a small side-channel via the drops table + custom fields approach.
        // Instead, we'll compute them now and store in the unit directly.

        let monsters = data::combat_monster_detail_map();
        let game_monster = monsters.get(&hrid)
            .unwrap_or_else(|| panic!("No monster found for hrid: {}", hrid));

        unit.enrage_time = game_monster["enrageTime"].as_i64().unwrap_or(0);

        let labyrinth_scale = room_level as f64 / LABYRINTH_MONSTER_BASE_ROOM_LEVEL;
        let is_guild_monster = game_monster["isGuildMonster"].as_bool().unwrap_or(false);
        // Guild monster data (combatMonsterDetailMap) is calibrated at difficulty_tier 100,
        // so abilities/levels scale off tier/100 instead of the labyrinth room level.
        let ability_scale = if is_guild_monster {
            difficulty_tier as f64 / 100.0
        } else {
            labyrinth_scale
        };

        // Abilities
        if let Some(abilities) = game_monster["abilities"].as_array() {
            for (i, ab) in abilities.iter().enumerate().take(5) {
                let min_tier = ab["minDifficultyTier"].as_i64().unwrap_or(0) as i32;
                if min_tier > difficulty_tier { continue; }
                let ab_hrid = ab["abilityHrid"].as_str().unwrap_or("").to_string();
                let level = (ab["level"].as_f64().unwrap_or(1.0) * ability_scale).floor() as i32;
                unit.abilities[i] = Some(Ability::new(ab_hrid, level.max(1), None));
            }
        }

        // Drop table
        if let Some(dt) = game_monster["dropTable"].as_array() {
            for item in dt {
                unit.combat_unit_drop_table_mut().push(Drops::new(
                    item["itemHrid"].as_str().unwrap_or("").to_string(),
                    item["dropRate"].as_f64().unwrap_or(0.0),
                    item["minCount"].as_i64().unwrap_or(0) as i32,
                    item["maxCount"].as_i64().unwrap_or(0) as i32,
                    item["difficultyTier"].as_i64().unwrap_or(0) as i32,
                ));
            }
        }

        // Rare drop table
        if let Some(rdt) = game_monster["rareDropTable"].as_array() {
            let dt_opt = game_monster["dropTable"].as_array();
            for (i, item) in rdt.iter().enumerate() {
                let dt_tier = dt_opt
                    .and_then(|dt| dt.get(i))
                    .and_then(|it| it["difficultyTier"].as_i64())
                    .unwrap_or_else(|| item["minDifficultyTier"].as_i64().unwrap_or(0));
                unit.combat_unit_rare_drop_table_mut().push(Drops::new(
                    item["itemHrid"].as_str().unwrap_or("").to_string(),
                    item["dropRate"].as_f64().unwrap_or(0.0),
                    item["minCount"].as_i64().unwrap_or(0) as i32,
                    item["maxCount"].as_i64().unwrap_or(0) as i32,
                    dt_tier as i32,
                ));
            }
        }

        // Store difficulty_tier and room_level for use in update_combat_details
        unit.monster_difficulty_tier = difficulty_tier;
        unit.monster_room_level = room_level;
        unit.monster_num_players = num_players;

        // Initial stats computation
        monster_update_combat_details(&mut unit);

        unit
    }
}

/// Called instead of unit.update_combat_details() for monsters
pub fn monster_update_combat_details(unit: &mut CombatUnit) {
    let hrid = unit.hrid.clone();
    let difficulty_tier = unit.monster_difficulty_tier;
    let room_level = unit.monster_room_level;

    let monsters = data::combat_monster_detail_map();
    let game_monster = monsters.get(&hrid)
        .unwrap_or_else(|| panic!("No monster found for hrid: {}", hrid));

    let is_guild_monster = game_monster["isGuildMonster"].as_bool().unwrap_or(false);
    let laby_scale = room_level as f64 / LABYRINTH_MONSTER_BASE_ROOM_LEVEL;

    let base = |field: &str| game_monster["combatDetails"][field].as_f64().unwrap_or(0.0);

    // Guild monster data (combatMonsterDetailMap) is calibrated at difficulty_tier 100,
    // where every level field already equals 100. Levels scale linearly off tier/100;
    // bonus-percent combat stats (accuracy%, damage%, evasion%, HP/MP ratio, etc.) are
    // copied unscaled below, and only armor/resistances scale explicitly with tier/100.
    let resist_scale = if is_guild_monster {
        let tier_scale = difficulty_tier as f64 / 100.0;
        unit.stamina_level = base("staminaLevel") * tier_scale;
        unit.intelligence_level = base("intelligenceLevel") * tier_scale;
        unit.attack_level = base("attackLevel") * tier_scale;
        unit.melee_level = base("meleeLevel") * tier_scale;
        unit.defense_level = base("defenseLevel") * tier_scale;
        unit.ranged_level = base("rangedLevel") * tier_scale;
        unit.magic_level = base("magicLevel") * tier_scale;

        unit.experience = game_monster["experience"].as_f64().unwrap_or(0.0);

        tier_scale
    } else {
        let level_mult = 1.0 + 0.25 * difficulty_tier as f64;
        let def_level_mult = 1.0 + 0.15 * difficulty_tier as f64;
        let level_bonus = 20.0 * difficulty_tier as f64;

        unit.stamina_level = level_mult * (base("staminaLevel") + level_bonus) * laby_scale;
        unit.intelligence_level = level_mult * (base("intelligenceLevel") + level_bonus) * laby_scale;
        unit.attack_level = level_mult * (base("attackLevel") + level_bonus) * laby_scale;
        unit.melee_level = level_mult * (base("meleeLevel") + level_bonus) * laby_scale;
        unit.defense_level = def_level_mult * (base("defenseLevel") + level_bonus) * laby_scale;
        unit.ranged_level = level_mult * (base("rangedLevel") + level_bonus) * laby_scale;
        unit.magic_level = level_mult * (base("magicLevel") + level_bonus) * laby_scale;

        let exp_mult = 1.0 + 0.5 * difficulty_tier as f64;
        let exp_bonus = 5.0 * difficulty_tier as f64;
        unit.experience = exp_mult * (game_monster["experience"].as_f64().unwrap_or(0.0) + exp_bonus);

        laby_scale
    };

    let monster_stats = &game_monster["combatDetails"]["combatStats"];

    // Combat style
    unit.combat_details.combat_stats.combat_style_hrid = monster_stats["combatStyleHrids"][0]
        .as_str()
        .unwrap_or("/combat_styles/smash")
        .to_string();

    // Copy all stats from monster data
    if let Some(obj) = monster_stats.as_object() {
        for (key, val) in obj {
            if key == "combatStyleHrids" { continue; }
            if let Some(v) = val.as_f64() {
                set_combat_stat_by_name(&mut unit.combat_details.combat_stats, key, v);
            }
        }
    }

    // Scale resistances (laby_scale for labyrinth monsters, tier/100 for guild monsters)
    unit.combat_details.combat_stats.armor *= resist_scale;
    unit.combat_details.combat_stats.water_resistance *= resist_scale;
    unit.combat_details.combat_stats.nature_resistance *= resist_scale;
    unit.combat_details.combat_stats.fire_resistance *= resist_scale;

    // Zero-out any missing stats
    for stat in MONSTER_STATS {
        if monster_stats[stat].is_null() {
            set_combat_stat_by_name(&mut unit.combat_details.combat_stats, stat, 0.0);
        }
    }

    // Use combatStats.attackInterval (the base value before level scaling).
    // update_combat_details_base will then divide by (1 + attackLevel/2000).
    // Only fall back to combatDetails.attackInterval if combatStats has no value (== 0).
    if unit.combat_details.combat_stats.attack_interval == 0.0 {
        let fallback = game_monster["combatDetails"]["attackInterval"].as_f64().unwrap_or(3_000_000_000.0);
        unit.combat_details.combat_stats.attack_interval = fallback;
    }

    // Save monster equipment-derived stats as baseline before buff application
    unit.base_combat_stats = unit.combat_details.combat_stats.clone();

    unit.update_combat_details_base();

    // Guild monsters get +1% max HP, +2% attack/cast speed, and +2 ability haste per player in the party.
    if is_guild_monster && unit.monster_num_players > 0 {
        let n = unit.monster_num_players as f64;

        let hp_scale = 1.0 + 0.01 * n;
        unit.combat_details.max_hitpoints =
            (unit.combat_details.max_hitpoints as f64 * hp_scale).floor() as i64;

        // attack_interval was already derived from the pre-bonus attack_speed in
        // update_combat_details_base(), so apply the bonus as an extra division here.
        let attack_speed_bonus = 0.02 * n;
        unit.combat_details.combat_stats.attack_interval /= 1.0 + attack_speed_bonus;
        unit.combat_details.combat_stats.attack_speed += attack_speed_bonus;
        unit.combat_details.combat_stats.cast_speed += 0.02 * n;
        unit.combat_details.combat_stats.ability_haste += 2.0 * n;
    }
}

// -- Extension fields we bolt onto CombatUnit for monsters --------------------
// Rather than a separate Monster struct (which would require Rc/Box) we use
// side-channel fields on CombatUnit that are only populated for monsters.

impl CombatUnit {
    pub fn combat_unit_drop_table_mut(&mut self) -> &mut Vec<Drops> {
        &mut self.drop_table
    }
    pub fn combat_unit_rare_drop_table_mut(&mut self) -> &mut Vec<Drops> {
        &mut self.rare_drop_table
    }
}
