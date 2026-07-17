use crate::combatsimulator::{buff::Buff, combat_unit::CombatUnit, data, monster::MonsterBuilder};

pub struct Labyrinth {
    pub monster_hrid: String,
    pub room_level: i32,
    pub buffs: Vec<Buff>,
    pub attempt_count: i32,
    pub encounter_start_time: i64,
}

impl Labyrinth {
    pub fn new(monster_hrid: String, room_level: i32, crates: Vec<String>) -> Self {
        let mut buffs = Vec::new();
        let crate_map = data::labyrinth_crate_detail_map();
        for crate_hrid in &crates {
            if let Some(crate_buffs) = crate_map.get(crate_hrid) {
                if let Some(arr) = crate_buffs.as_array() {
                    for b in arr {
                        buffs.push(Buff::from_value(b, 1));
                    }
                }
            }
        }

        Labyrinth {
            monster_hrid,
            room_level,
            buffs,
            attempt_count: 0,
            encounter_start_time: 0,
        }
    }

    pub fn get_monster(&mut self) -> Vec<CombatUnit> {
        self.attempt_count += 1;
        vec![MonsterBuilder::new(self.monster_hrid.clone(), 0, self.room_level, 0)]
    }

    pub fn update_encounter_start_time(&mut self, t: i64) {
        self.encounter_start_time = t;
    }

    pub fn check_timeout(&self, current_time: i64) -> bool {
        current_time - self.encounter_start_time > 120 * 1_000_000_000
    }
}
