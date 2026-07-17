use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::combatsimulator::{buff::Buff, trigger::Trigger, data};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbilityEffect {
    pub target_type: String,
    pub effect_type: String,
    pub combat_style_hrid: String,
    pub damage_type: String,
    pub damage_flat: f64,
    pub damage_ratio: f64,
    pub bonus_accuracy_ratio: f64,
    pub damage_over_time_ratio: f64,
    pub damage_over_time_duration: i64,
    pub armor_damage_ratio: f64,
    pub hp_drain_ratio: f64,
    pub pierce_chance: f64,
    pub blind_chance: f64,
    pub blind_duration: i64,
    pub silence_chance: f64,
    pub silence_duration: i64,
    pub stun_chance: f64,
    pub stun_duration: i64,
    pub spend_hp_ratio: f64,
    pub buffs: Option<Vec<Buff>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ability {
    pub hrid: String,
    pub level: i32,
    pub mana_cost: i64,
    pub cooldown_duration: i64,
    pub cast_duration: i64,
    pub is_special_ability: bool,
    pub ability_effects: Vec<AbilityEffect>,
    pub triggers: Vec<Trigger>,
    pub last_used: i64,
}

// Compile-time pseudo-abilities generated from combat stats (blaze, bloom)
fn ability_from_combat_stat(name: &str) -> Option<Value> {
    match name {
        "blaze" => Some(serde_json::json!({
            "hrid": "/abilities/blaze",
            "name": "Blaze",
            "isSpecialAbility": false,
            "manaCost": 0,
            "cooldownDuration": 0,
            "castDuration": 0,
            "abilityEffects": [{
                "targetType": "allEnemies",
                "effectType": "/ability_effect_types/damage",
                "combatStyleHrid": "/combat_styles/magic",
                "damageType": "/damage_types/fire",
                "baseDamageFlat": 0,
                "baseDamageFlatLevelBonus": 0.0,
                "baseDamageRatio": 0.3,
                "baseDamageRatioLevelBonus": 0,
                "bonusAccuracyRatio": 0,
                "bonusAccuracyRatioLevelBonus": 0,
                "damageOverTimeRatio": 0,
                "damageOverTimeDuration": 0,
                "armorDamageRatio": 0,
                "armorDamageRatioLevelBonus": 0,
                "hpDrainRatio": 0,
                "pierceChance": 0,
                "blindChance": 0,
                "blindDuration": 0,
                "silenceChance": 0,
                "silenceDuration": 0,
                "stunChance": 0,
                "stunDuration": 0,
                "spendHpRatio": 0,
                "buffs": null
            }],
            "defaultCombatTriggers": [
                {
                    "dependencyHrid": "/combat_trigger_dependencies/all_enemies",
                    "conditionHrid": "/combat_trigger_conditions/number_of_active_units",
                    "comparatorHrid": "/combat_trigger_comparators/greater_than_equal",
                    "value": 1
                },
                {
                    "dependencyHrid": "/combat_trigger_dependencies/all_enemies",
                    "conditionHrid": "/combat_trigger_conditions/current_hp",
                    "comparatorHrid": "/combat_trigger_comparators/greater_than_equal",
                    "value": 1
                }
            ]
        })),
        "bloom" => Some(serde_json::json!({
            "hrid": "/abilities/bloom",
            "name": "Bloom",
            "isSpecialAbility": false,
            "manaCost": 0,
            "cooldownDuration": 0,
            "castDuration": 0,
            "abilityEffects": [{
                "targetType": "lowestHpAlly",
                "effectType": "/ability_effect_types/heal",
                "combatStyleHrid": "/combat_styles/magic",
                "damageType": "",
                "baseDamageFlat": 10,
                "baseDamageFlatLevelBonus": 0,
                "baseDamageRatio": 0.15,
                "baseDamageRatioLevelBonus": 0,
                "bonusAccuracyRatio": 0,
                "bonusAccuracyRatioLevelBonus": 0,
                "damageOverTimeRatio": 0,
                "damageOverTimeDuration": 0,
                "armorDamageRatio": 0,
                "armorDamageRatioLevelBonus": 0,
                "hpDrainRatio": 0,
                "pierceChance": 0,
                "blindChance": 0,
                "blindDuration": 0,
                "silenceChance": 0,
                "silenceDuration": 0,
                "stunChance": 0,
                "stunDuration": 0,
                "spendHpRatio": 0,
                "buffs": null
            }],
            "defaultCombatTriggers": [
                {
                    "dependencyHrid": "/combat_trigger_dependencies/all_allies",
                    "conditionHrid": "/combat_trigger_conditions/lowest_hp_percentage",
                    "comparatorHrid": "/combat_trigger_comparators/less_than_equal",
                    "value": 100
                }
            ]
        })),
        _ => None,
    }
}

impl Ability {
    pub fn new(hrid: String, level: i32, triggers_opt: Option<Vec<Trigger>>) -> Self {
        let game_ability = data::ability_detail_map().get(&hrid)
            .cloned()
            .or_else(|| {
                let short = hrid.split('/').last().unwrap_or("");
                ability_from_combat_stat(short)
            })
            .unwrap_or_else(|| panic!("No ability found for hrid: {}", hrid));

        let mana_cost = game_ability["manaCost"].as_i64().unwrap_or(0);
        let cooldown_duration = game_ability["cooldownDuration"].as_i64().unwrap_or(0);
        let cast_duration = game_ability["castDuration"].as_i64().unwrap_or(0);
        let is_special_ability = game_ability["isSpecialAbility"].as_bool().unwrap_or(false);

        let mut ability_effects = Vec::new();
        if let Some(effects) = game_ability["abilityEffects"].as_array() {
            for effect in effects {
                let fl = effect["baseDamageFlat"].as_f64().unwrap_or(0.0)
                    + (level - 1) as f64 * effect["baseDamageFlatLevelBonus"].as_f64().unwrap_or(0.0);
                let ratio = effect["baseDamageRatio"].as_f64().unwrap_or(0.0)
                    + (level - 1) as f64 * effect["baseDamageRatioLevelBonus"].as_f64().unwrap_or(0.0);
                let bonus_acc = effect["bonusAccuracyRatio"].as_f64().unwrap_or(0.0)
                    + (level - 1) as f64 * effect["bonusAccuracyRatioLevelBonus"].as_f64().unwrap_or(0.0);
                let armor_dmg = effect["armorDamageRatio"].as_f64().unwrap_or(0.0)
                    + (level - 1) as f64 * effect["armorDamageRatioLevelBonus"].as_f64().unwrap_or(0.0);

                let buffs = if effect["buffs"].is_array() {
                    let bv: Vec<Buff> = effect["buffs"].as_array().unwrap()
                        .iter()
                        .map(|b| Buff::from_value(b, level))
                        .collect();
                    Some(bv)
                } else {
                    None
                };

                ability_effects.push(AbilityEffect {
                    target_type: effect["targetType"].as_str().unwrap_or("").to_string(),
                    effect_type: effect["effectType"].as_str().unwrap_or("").to_string(),
                    combat_style_hrid: effect["combatStyleHrid"].as_str().unwrap_or("").to_string(),
                    damage_type: effect["damageType"].as_str().unwrap_or("").to_string(),
                    damage_flat: fl,
                    damage_ratio: ratio,
                    bonus_accuracy_ratio: bonus_acc,
                    damage_over_time_ratio: effect["damageOverTimeRatio"].as_f64().unwrap_or(0.0),
                    damage_over_time_duration: effect["damageOverTimeDuration"].as_i64().unwrap_or(0),
                    armor_damage_ratio: armor_dmg,
                    hp_drain_ratio: effect["hpDrainRatio"].as_f64().unwrap_or(0.0),
                    pierce_chance: effect["pierceChance"].as_f64().unwrap_or(0.0),
                    blind_chance: effect["blindChance"].as_f64().unwrap_or(0.0),
                    blind_duration: effect["blindDuration"].as_i64().unwrap_or(0),
                    silence_chance: effect["silenceChance"].as_f64().unwrap_or(0.0),
                    silence_duration: effect["silenceDuration"].as_i64().unwrap_or(0),
                    stun_chance: effect["stunChance"].as_f64().unwrap_or(0.0),
                    stun_duration: effect["stunDuration"].as_i64().unwrap_or(0),
                    spend_hp_ratio: effect["spendHpRatio"].as_f64().unwrap_or(0.0),
                    buffs,
                });
            }
        }

        let triggers = if let Some(t) = triggers_opt {
            t
        } else {
            game_ability["defaultCombatTriggers"]
                .as_array()
                .map(|arr| arr.iter().map(|t| Trigger::from_dto(t)).collect())
                .unwrap_or_default()
        };

        Ability {
            hrid,
            level,
            mana_cost,
            cooldown_duration,
            cast_duration,
            is_special_ability,
            ability_effects,
            triggers,
            last_used: i64::MIN,
        }
    }

    pub fn from_dto(dto: &Value) -> Self {
        let hrid = dto["hrid"].as_str().unwrap_or("").to_string();
        let level = dto["level"].as_i64().unwrap_or(1) as i32;
        let triggers: Vec<Trigger> = dto["triggers"].as_array()
            .map(|arr| arr.iter().map(|t| Trigger::from_dto(t)).collect())
            .unwrap_or_default();
        Ability::new(hrid, level, Some(triggers))
    }

    /// Returns whether the ability should trigger, given current state. Actual trigger evaluation
    /// with full unit context is in CombatSimulator; this just checks cooldown and basic status.
    pub fn can_be_used(&self, current_time: i64, is_stunned: bool, is_silenced: bool) -> bool {
        if is_stunned || is_silenced {
            return false;
        }
        // Cooldown check without haste (haste applied by caller)
        self.last_used.saturating_add(self.cooldown_duration) <= current_time
    }
}
