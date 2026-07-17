use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::combatsimulator::data;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Equipment {
    pub hrid: String,
    pub enhancement_level: usize,
}

impl Equipment {
    pub fn new(hrid: String, enhancement_level: usize) -> Self {
        // Validate
        let _ = data::item_detail_map().get(&hrid)
            .unwrap_or_else(|| panic!("No equipment found for hrid: {}", hrid));
        Equipment { hrid, enhancement_level }
    }

    pub fn from_dto(dto: &Value) -> Self {
        let hrid = dto["hrid"].as_str().unwrap_or("").to_string();
        let level = dto["enhancementLevel"].as_u64().unwrap_or(0) as usize;
        Equipment::new(hrid, level)
    }

    fn game_item(&self) -> &Value {
        data::item_detail_map().get(&self.hrid).unwrap()
    }

    pub fn get_combat_stat(&self, stat: &str) -> f64 {
        let item = self.game_item();
        let detail = &item["equipmentDetail"];
        let val = detail["combatStats"][stat].as_f64().unwrap_or(0.0);
        if val != 0.0 {
            let table = data::enhancement_level_table();
            let multiplier = table.get(self.enhancement_level).copied().unwrap_or(0.0);
            let bonus = detail["combatEnhancementBonuses"][stat].as_f64().unwrap_or(0.0);
            val + multiplier * bonus
        } else {
            0.0
        }
    }

    pub fn get_combat_style(&self) -> String {
        let item = self.game_item();
        item["equipmentDetail"]["combatStats"]["combatStyleHrids"][0]
            .as_str()
            .unwrap_or("/combat_styles/smash")
            .to_string()
    }

    pub fn get_damage_type(&self) -> String {
        let item = self.game_item();
        item["equipmentDetail"]["combatStats"]["damageType"]
            .as_str()
            .unwrap_or("/damage_types/physical")
            .to_string()
    }

    pub fn get_primary_training(&self) -> String {
        let item = self.game_item();
        item["equipmentDetail"]["combatStats"]["primaryTraining"]
            .as_str()
            .unwrap_or("")
            .to_string()
    }

    pub fn get_focus_training(&self) -> String {
        let item = self.game_item();
        item["equipmentDetail"]["combatStats"]["focusTraining"]
            .as_str()
            .unwrap_or("")
            .to_string()
    }
}
