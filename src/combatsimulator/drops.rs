use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Drops {
    pub item_hrid: String,
    pub drop_rate: f64,
    pub min_count: i32,
    pub max_count: i32,
    pub difficulty_tier: i32,
}

impl Drops {
    pub fn new(item_hrid: String, drop_rate: f64, min_count: i32, max_count: i32, difficulty_tier: i32) -> Self {
        Drops { item_hrid, drop_rate, min_count, max_count, difficulty_tier }
    }
}
