use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Buff {
    pub unique_hrid: String,
    pub type_hrid: String,
    pub ratio_boost: f64,
    pub flat_boost: f64,
    pub duration: i64,
    pub multiplier_for_skill_hrid: String,
    pub multiplier_per_skill_level: f64,
    pub start_time: i64,
}

impl Buff {
    pub fn from_value(v: &Value, level: i32) -> Self {
        let ratio_boost = v["ratioBoost"].as_f64().unwrap_or(0.0)
            + (level - 1) as f64 * v["ratioBoostLevelBonus"].as_f64().unwrap_or(0.0);
        let flat_boost = v["flatBoost"].as_f64().unwrap_or(0.0)
            + (level - 1) as f64 * v["flatBoostLevelBonus"].as_f64().unwrap_or(0.0);

        Buff {
            unique_hrid: v["uniqueHrid"].as_str().unwrap_or("").to_string(),
            type_hrid: v["typeHrid"].as_str().unwrap_or("").to_string(),
            ratio_boost,
            flat_boost,
            duration: v["duration"].as_i64().unwrap_or(0),
            multiplier_for_skill_hrid: v["multiplierForSkillHrid"].as_str().unwrap_or("").to_string(),
            multiplier_per_skill_level: v["multiplierPerSkillLevel"].as_f64().unwrap_or(0.0),
            start_time: 0,
        }
    }

    pub fn inline(unique_hrid: &str, type_hrid: &str, ratio_boost: f64, flat_boost: f64, duration: i64) -> Self {
        Buff {
            unique_hrid: unique_hrid.to_string(),
            type_hrid: type_hrid.to_string(),
            ratio_boost,
            flat_boost,
            duration,
            multiplier_for_skill_hrid: String::new(),
            multiplier_per_skill_level: 0.0,
            start_time: 0,
        }
    }
}
