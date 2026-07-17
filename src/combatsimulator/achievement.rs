use serde_json::Value;
use crate::combatsimulator::{buff::Buff, data};

pub struct Achievement;

impl Achievement {
    /// Returns combat-applicable buff contributions from a player's completed achievements.
    ///
    /// Mirrors the JS logic exactly:
    ///   For each achievement tier, check if ALL achievements of that tier are completed.
    ///   If yes, add ONE buff for that tier (not count * buff).
    ///   If no, add nothing for that tier.
    pub fn get_buffs(achievements_dto: &Value) -> Vec<Buff> {
        let ach_map  = data::achievement_detail_map();
        let tier_map = data::achievement_tier_detail_map();

        let completed: std::collections::HashSet<&str> = achievements_dto
            .as_object()
            .map(|obj| obj.iter()
                .filter(|(_, v)| v.as_bool().unwrap_or(false))
                .map(|(k, _)| k.as_str())
                .collect())
            .unwrap_or_default();

        let mut buffs = Vec::new();

        for (tier_hrid, tier) in tier_map.iter() {
            // Check if the buff applies to combat
            let usable_in_combat = tier["usableInActionTypeMap"]["/action_types/combat"]
                .as_bool()
                .unwrap_or(false);
            if !usable_in_combat { continue; }

            // Check if ALL achievements of this tier are completed
            let all_in_tier: Vec<&str> = ach_map.values()
                .filter(|a| a["tierHrid"].as_str() == Some(tier_hrid.as_str()))
                .filter_map(|a| a["hrid"].as_str())
                .collect();

            if all_in_tier.is_empty() { continue; }

            let all_done = all_in_tier.iter().all(|hrid| completed.contains(*hrid));
            if !all_done { continue; }

            let buff_data = &tier["buff"];
            let unique_hrid = buff_data["uniqueHrid"].as_str().unwrap_or("").to_string();
            let type_hrid   = buff_data["typeHrid"].as_str().unwrap_or("").to_string();
            let flat_boost  = buff_data["flatBoost"].as_f64().unwrap_or(0.0);
            let ratio_boost = buff_data["ratioBoost"].as_f64().unwrap_or(0.0);

            if unique_hrid.is_empty() || type_hrid.is_empty() { continue; }

            buffs.push(Buff {
                unique_hrid,
                type_hrid,
                flat_boost,
                ratio_boost,
                duration: 0,
                multiplier_for_skill_hrid: String::new(),
                multiplier_per_skill_level: 0.0,
                start_time: 0,
            });
        }

        buffs
    }
}
