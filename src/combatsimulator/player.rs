use std::collections::HashMap;
use serde_json::Value;
use crate::combatsimulator::{
    ability::Ability,
    achievement::Achievement,
    combat_unit::CombatUnit,
    consumable::Consumable,
    equipment::Equipment,
    house_room::HouseRoom,
};

const EQUIPMENT_SLOTS: &[&str] = &[
    "/equipment_types/head",
    "/equipment_types/body",
    "/equipment_types/legs",
    "/equipment_types/feet",
    "/equipment_types/hands",
    "/equipment_types/main_hand",
    "/equipment_types/two_hand",
    "/equipment_types/off_hand",
    "/equipment_types/pouch",
    "/equipment_types/back",
    "/equipment_types/charm",
    "/equipment_types/neck",
    "/equipment_types/ring",
    "/equipment_types/earrings",
];

const EQUIPMENT_STATS: &[&str] = &[
    "stabAccuracy", "slashAccuracy", "smashAccuracy", "rangedAccuracy", "magicAccuracy",
    "stabDamage", "slashDamage", "smashDamage", "rangedDamage", "magicDamage",
    "defensiveDamage", "taskDamage",
    "physicalAmplify", "waterAmplify", "natureAmplify", "fireAmplify", "healingAmplify",
    "stabEvasion", "slashEvasion", "smashEvasion", "rangedEvasion", "magicEvasion",
    "armor", "waterResistance", "natureResistance", "fireResistance",
    "maxHitpoints", "maxManapoints",
    "lifeSteal", "hpRegenPer10", "mpRegenPer10",
    "physicalThorns", "elementalThorns",
    "combatDropRate", "combatRareFind", "combatDropQuantity", "combatExperience",
    "criticalRate", "criticalDamage",
    "armorPenetration", "waterPenetration", "naturePenetration", "firePenetration",
    "abilityHaste", "tenacity",
    "manaLeech", "castSpeed",
    "threat", "parry", "mayhem", "pierce", "curse", "fury", "weaken", "ripple", "bloom", "blaze",
    "attackSpeed", "foodHaste", "drinkConcentration",
    "autoAttackDamage", "abilityDamage",
    "staminaExperience", "intelligenceExperience", "attackExperience",
    "defenseExperience", "meleeExperience", "rangedExperience", "magicExperience",
    "retaliation",
];

pub type Player = CombatUnit;

pub trait PlayerExt {
    fn create_from_dto(dto: &Value) -> CombatUnit;
    fn player_update_combat_details(&mut self);
}

impl PlayerExt for CombatUnit {
    fn create_from_dto(dto: &Value) -> CombatUnit {
        let mut unit = CombatUnit::new_base();
        unit.is_player = true;
        unit.hrid = dto["hrid"].as_str().unwrap_or("player").to_string();

        unit.stamina_level = dto["staminaLevel"].as_f64().unwrap_or(1.0);
        unit.intelligence_level = dto["intelligenceLevel"].as_f64().unwrap_or(1.0);
        unit.attack_level = dto["attackLevel"].as_f64().unwrap_or(1.0);
        unit.melee_level = dto["meleeLevel"].as_f64().unwrap_or(1.0);
        unit.defense_level = dto["defenseLevel"].as_f64().unwrap_or(1.0);
        unit.ranged_level = dto["rangedLevel"].as_f64().unwrap_or(1.0);
        unit.magic_level = dto["magicLevel"].as_f64().unwrap_or(1.0);

        // Equipment
        for slot in EQUIPMENT_SLOTS {
            let val = &dto["equipment"][slot];
            let eq = if val.is_object() {
                Some(Equipment::from_dto(val))
            } else {
                None
            };
            unit.equipment.insert(slot.to_string(), eq);
        }

        // Food
        if let Some(food_arr) = dto["food"].as_array() {
            for (i, f) in food_arr.iter().enumerate().take(3) {
                unit.food[i] = if f.is_object() {
                    Some(Consumable::from_dto(f))
                } else {
                    None
                };
            }
        }

        // Drinks
        if let Some(drinks_arr) = dto["drinks"].as_array() {
            for (i, d) in drinks_arr.iter().enumerate().take(3) {
                unit.drinks[i] = if d.is_object() {
                    Some(Consumable::from_dto(d))
                } else {
                    None
                };
            }
        }

        // Abilities
        if let Some(abil_arr) = dto["abilities"].as_array() {
            for (i, a) in abil_arr.iter().enumerate().take(5) {
                unit.abilities[i] = if a.is_object() {
                    Some(Ability::from_dto(a))
                } else {
                    None
                };
            }
        }

        // House rooms
        if let Some(rooms_obj) = dto["houseRooms"].as_object() {
            for (hrid, level_val) in rooms_obj {
                let level = level_val.as_i64().unwrap_or(0) as i32;
                if level > 0 {
                    unit.house_rooms.push(HouseRoom::new(hrid.clone(), level));
                }
            }
        }

        // Achievements — one buff contribution per completed achievement based on its tier
        for buff in &Achievement::get_buffs(&dto["achievements"]) {
            unit.add_permanent_buff(buff);
        }

        unit.debuff_on_level_gap = dto["debuffOnLevelGap"].as_f64().unwrap_or(0.0);

        unit
    }

    fn player_update_combat_details(&mut self) {
        // Determine weapon stats
        if let Some(Some(main_hand)) = self.equipment.get("/equipment_types/main_hand").cloned() {
            self.combat_details.combat_stats.combat_style_hrid = main_hand.get_combat_style();
            self.combat_details.combat_stats.damage_type = main_hand.get_damage_type();
            self.combat_details.combat_stats.attack_interval = main_hand.get_combat_stat("attackInterval");
            self.combat_details.combat_stats.primary_training = main_hand.get_primary_training();
        } else if let Some(Some(two_hand)) = self.equipment.get("/equipment_types/two_hand").cloned() {
            self.combat_details.combat_stats.combat_style_hrid = two_hand.get_combat_style();
            self.combat_details.combat_stats.damage_type = two_hand.get_damage_type();
            self.combat_details.combat_stats.attack_interval = two_hand.get_combat_stat("attackInterval");
            self.combat_details.combat_stats.primary_training = two_hand.get_primary_training();
        } else {
            self.combat_details.combat_stats.combat_style_hrid = "/combat_styles/smash".to_string();
            self.combat_details.combat_stats.damage_type = "/damage_types/physical".to_string();
            self.combat_details.combat_stats.attack_interval = 3_000_000_000.0;
            self.combat_details.combat_stats.primary_training = "/skills/melee".to_string();
        }

        if let Some(Some(charm)) = self.equipment.get("/equipment_types/charm").cloned() {
            self.combat_details.combat_stats.focus_training = charm.get_focus_training();
        } else {
            self.combat_details.combat_stats.focus_training = String::new();
        }

        // Sum equipment stats
        let equipment_clone: Vec<_> = self.equipment.values()
            .filter_map(|e| e.clone())
            .collect();

        fn snake_stat(camel: &str) -> &str { camel }

        for stat_camel in EQUIPMENT_STATS {
            let sum: f64 = equipment_clone.iter()
                .map(|eq| eq.get_combat_stat(stat_camel))
                .sum();
            set_combat_stat_by_name(&mut self.combat_details.combat_stats, stat_camel, sum);
        }

        // Food/drink slots
        if let Some(Some(pouch)) = self.equipment.get("/equipment_types/pouch").cloned() {
            self.combat_details.combat_stats.food_slots = 1.0 + pouch.get_combat_stat("foodSlots");
            self.combat_details.combat_stats.drink_slots = 1.0 + pouch.get_combat_stat("drinkSlots");
        } else {
            self.combat_details.combat_stats.food_slots = 1.0;
            self.combat_details.combat_stats.drink_slots = 1.0;
        }
        // Save equipment-derived stats as baseline before buff application
        self.base_combat_stats = self.combat_details.combat_stats.clone();

        self.update_combat_details_base();
    }
}

/// Map camelCase stat names (from JSON data) to CombatStats fields
pub fn set_combat_stat_by_name(stats: &mut crate::combatsimulator::combat_unit::CombatStats, name: &str, val: f64) {
    match name {
        "stabAccuracy" => stats.stab_accuracy = val,
        "slashAccuracy" => stats.slash_accuracy = val,
        "smashAccuracy" => stats.smash_accuracy = val,
        "rangedAccuracy" => stats.ranged_accuracy = val,
        "magicAccuracy" => stats.magic_accuracy = val,
        "stabDamage" => stats.stab_damage = val,
        "slashDamage" => stats.slash_damage = val,
        "smashDamage" => stats.smash_damage = val,
        "rangedDamage" => stats.ranged_damage = val,
        "magicDamage" => stats.magic_damage = val,
        "defensiveDamage" => stats.defensive_damage = val,
        "taskDamage" => stats.task_damage = val,
        "physicalAmplify" => stats.physical_amplify = val,
        "waterAmplify" => stats.water_amplify = val,
        "natureAmplify" => stats.nature_amplify = val,
        "fireAmplify" => stats.fire_amplify = val,
        "healingAmplify" => stats.healing_amplify = val,
        "stabEvasion" => stats.stab_evasion = val,
        "slashEvasion" => stats.slash_evasion = val,
        "smashEvasion" => stats.smash_evasion = val,
        "rangedEvasion" => stats.ranged_evasion = val,
        "magicEvasion" => stats.magic_evasion = val,
        "armor" => stats.armor = val,
        "waterResistance" => stats.water_resistance = val,
        "natureResistance" => stats.nature_resistance = val,
        "fireResistance" => stats.fire_resistance = val,
        "maxHitpoints" => stats.max_hitpoints = val,
        "maxManapoints" => stats.max_manapoints = val,
        "maxHitpointsRatio" => stats.max_hitpoints_ratio = val,
        "maxManapointsRatio" => stats.max_manapoints_ratio = val,
        "lifeSteal" => stats.life_steal = val,
        "hpRegenPer10" => stats.hp_regen_per10 = val,
        "mpRegenPer10" => stats.mp_regen_per10 = val,
        "physicalThorns" => stats.physical_thorns = val,
        "elementalThorns" => stats.elemental_thorns = val,
        "combatDropRate" => stats.combat_drop_rate = val,
        "combatRareFind" => stats.combat_rare_find = val,
        "combatDropQuantity" => stats.combat_drop_quantity = val,
        "combatExperience" => stats.combat_experience = val,
        "criticalRate" => stats.critical_rate = val,
        "criticalDamage" => stats.critical_damage = val,
        "armorPenetration" => stats.armor_penetration = val,
        "waterPenetration" => stats.water_penetration = val,
        "naturePenetration" => stats.nature_penetration = val,
        "firePenetration" => stats.fire_penetration = val,
        "abilityHaste" => stats.ability_haste = val,
        "tenacity" => stats.tenacity = val,
        "manaLeech" => stats.mana_leech = val,
        "castSpeed" => stats.cast_speed = val,
        "threat" => stats.threat = val,
        "parry" => stats.parry = val,
        "mayhem" => stats.mayhem = val,
        "pierce" => stats.pierce = val,
        "curse" => stats.curse = val,
        "fury" => stats.fury = val,
        "weaken" => stats.weaken = val,
        "ripple" => stats.ripple = val,
        "bloom" => stats.bloom = val,
        "blaze" => stats.blaze = val,
        "attackSpeed" => stats.attack_speed = val,
        "foodHaste" => stats.food_haste = val,
        "drinkConcentration" => stats.drink_concentration = val,
        "autoAttackDamage" => stats.auto_attack_damage = val,
        "abilityDamage" => stats.ability_damage = val,
        "staminaExperience" => stats.stamina_experience = val,
        "intelligenceExperience" => stats.intelligence_experience = val,
        "attackExperience" => stats.attack_experience = val,
        "defenseExperience" => stats.defense_experience = val,
        "meleeExperience" => stats.melee_experience = val,
        "rangedExperience" => stats.ranged_experience = val,
        "magicExperience" => stats.magic_experience = val,
        "retaliation" => stats.retaliation = val,
        "attackInterval" => stats.attack_interval = val,
        _ => {}
    }
}
