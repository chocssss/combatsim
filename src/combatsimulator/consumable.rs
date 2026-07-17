use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::combatsimulator::{buff::Buff, trigger::Trigger, data};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Consumable {
    pub hrid: String,
    pub cooldown_duration: i64,
    pub hitpoint_restore: i64,
    pub manapoint_restore: i64,
    pub recovery_duration: i64,
    pub category_hrid: String,
    pub buffs: Vec<Buff>,
    pub triggers: Vec<Trigger>,
    pub last_used: i64,
}

impl Consumable {
    pub fn new(hrid: String, triggers_opt: Option<Vec<Trigger>>) -> Self {
        let items = data::item_detail_map();
        let game_consumable = items.get(&hrid)
            .unwrap_or_else(|| panic!("No consumable found for hrid: {}", hrid));

        let detail = &game_consumable["consumableDetail"];
        let cooldown_duration = detail["cooldownDuration"].as_i64().unwrap_or(0);
        let hitpoint_restore = detail["hitpointRestore"].as_i64().unwrap_or(0);
        let manapoint_restore = detail["manapointRestore"].as_i64().unwrap_or(0);
        let recovery_duration = detail["recoveryDuration"].as_i64().unwrap_or(0);
        let category_hrid = game_consumable["categoryHrid"].as_str().unwrap_or("").to_string();

        let mut buffs = Vec::new();
        if let Some(buff_arr) = detail["buffs"].as_array() {
            for b in buff_arr {
                buffs.push(Buff::from_value(b, 1));
            }
        }

        let triggers = if let Some(t) = triggers_opt {
            t
        } else {
            detail["defaultCombatTriggers"]
                .as_array()
                .map(|arr| arr.iter().map(|t| Trigger::from_dto(t)).collect())
                .unwrap_or_default()
        };

        Consumable {
            hrid,
            cooldown_duration,
            hitpoint_restore,
            manapoint_restore,
            recovery_duration,
            category_hrid,
            buffs,
            triggers,
            last_used: i64::MIN,
        }
    }

    pub fn from_dto(dto: &Value) -> Self {
        let hrid = dto["hrid"].as_str().unwrap_or("").to_string();
        let triggers: Vec<Trigger> = dto["triggers"].as_array()
            .map(|arr| arr.iter().map(|t| Trigger::from_dto(t)).collect())
            .unwrap_or_default();
        Consumable::new(hrid, Some(triggers))
    }

    pub fn is_food(&self) -> bool {
        self.category_hrid.contains("food")
    }

    pub fn is_drink(&self) -> bool {
        self.category_hrid.contains("drink")
    }
}
