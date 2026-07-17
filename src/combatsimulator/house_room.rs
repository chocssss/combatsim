use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::combatsimulator::{buff::Buff, data};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HouseRoom {
    pub hrid: String,
    pub level: i32,
    pub buffs: Vec<Buff>,
}

impl HouseRoom {
    pub fn new(hrid: String, level: i32) -> Self {
        let rooms = data::house_room_detail_map();
        let game_room = rooms.get(&hrid)
            .unwrap_or_else(|| panic!("No house room found for hrid: {}", hrid));

        let mut buffs = Vec::new();
        for key in &["actionBuffs", "globalBuffs"] {
            if let Some(arr) = game_room[key].as_array() {
                for b in arr {
                    buffs.push(Buff::from_value(b, level));
                }
            }
        }

        HouseRoom { hrid, level, buffs }
    }
}
