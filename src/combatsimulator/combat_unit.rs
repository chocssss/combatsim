use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::combatsimulator::{
    ability::Ability,
    buff::Buff,
    consumable::Consumable,
    house_room::HouseRoom,
};

// -- Combat Stats -------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombatStats {
    pub combat_style_hrid: String,
    pub damage_type: String,
    pub attack_interval: f64,
    pub auto_attack_damage: f64,
    pub ability_damage: f64,
    pub critical_rate: f64,
    pub critical_damage: f64,
    pub stab_accuracy: f64,
    pub slash_accuracy: f64,
    pub smash_accuracy: f64,
    pub ranged_accuracy: f64,
    pub magic_accuracy: f64,
    pub stab_damage: f64,
    pub slash_damage: f64,
    pub smash_damage: f64,
    pub ranged_damage: f64,
    pub magic_damage: f64,
    pub defensive_damage: f64,
    pub task_damage: f64,
    pub physical_amplify: f64,
    pub water_amplify: f64,
    pub nature_amplify: f64,
    pub fire_amplify: f64,
    pub healing_amplify: f64,
    pub physical_thorns: f64,
    pub elemental_thorns: f64,
    pub max_hitpoints: f64,
    pub max_manapoints: f64,
    pub stab_evasion: f64,
    pub slash_evasion: f64,
    pub smash_evasion: f64,
    pub ranged_evasion: f64,
    pub magic_evasion: f64,
    pub armor: f64,
    pub water_resistance: f64,
    pub nature_resistance: f64,
    pub fire_resistance: f64,
    pub life_steal: f64,
    pub hp_regen_per10: f64,
    pub mp_regen_per10: f64,
    pub combat_drop_rate: f64,
    pub combat_drop_quantity: f64,
    pub combat_rare_find: f64,
    pub combat_experience: f64,
    pub food_slots: f64,
    pub drink_slots: f64,
    pub armor_penetration: f64,
    pub water_penetration: f64,
    pub nature_penetration: f64,
    pub fire_penetration: f64,
    pub mana_leech: f64,
    pub cast_speed: f64,
    pub threat: f64,
    pub parry: f64,
    pub mayhem: f64,
    pub pierce: f64,
    pub curse: f64,
    pub ripple: f64,
    pub bloom: f64,
    pub blaze: f64,
    pub weaken: f64,
    pub fury: f64,
    pub food_haste: f64,
    pub drink_concentration: f64,
    pub damage_taken: f64,
    pub attack_speed: f64,
    pub armor_damage_ratio: f64,
    pub hp_drain_ratio: f64,
    pub primary_training: String,
    pub focus_training: String,
    pub stamina_experience: f64,
    pub intelligence_experience: f64,
    pub attack_experience: f64,
    pub defense_experience: f64,
    pub melee_experience: f64,
    pub ranged_experience: f64,
    pub magic_experience: f64,
    pub retaliation: f64,
    pub max_hitpoints_ratio: f64,
    pub max_manapoints_ratio: f64,
    pub ability_haste: f64,
    pub tenacity: f64,
}

impl Default for CombatStats {
    fn default() -> Self {
        CombatStats {
            combat_style_hrid: "/combat_styles/smash".to_string(),
            damage_type: "/damage_types/physical".to_string(),
            attack_interval: 3_000_000_000.0,
            auto_attack_damage: 0.0,
            ability_damage: 0.0,
            critical_rate: 0.0,
            critical_damage: 0.0,
            stab_accuracy: 0.0,
            slash_accuracy: 0.0,
            smash_accuracy: 0.0,
            ranged_accuracy: 0.0,
            magic_accuracy: 0.0,
            stab_damage: 0.0,
            slash_damage: 0.0,
            smash_damage: 0.0,
            ranged_damage: 0.0,
            magic_damage: 0.0,
            defensive_damage: 0.0,
            task_damage: 0.0,
            physical_amplify: 0.0,
            water_amplify: 0.0,
            nature_amplify: 0.0,
            fire_amplify: 0.0,
            healing_amplify: 0.0,
            physical_thorns: 0.0,
            elemental_thorns: 0.0,
            max_hitpoints: 0.0,
            max_manapoints: 0.0,
            stab_evasion: 0.0,
            slash_evasion: 0.0,
            smash_evasion: 0.0,
            ranged_evasion: 0.0,
            magic_evasion: 0.0,
            armor: 0.0,
            water_resistance: 0.0,
            nature_resistance: 0.0,
            fire_resistance: 0.0,
            life_steal: 0.0,
            hp_regen_per10: 0.01,
            mp_regen_per10: 0.01,
            combat_drop_rate: 0.0,
            combat_drop_quantity: 0.0,
            combat_rare_find: 0.0,
            combat_experience: 0.0,
            food_slots: 1.0,
            drink_slots: 1.0,
            armor_penetration: 0.0,
            water_penetration: 0.0,
            nature_penetration: 0.0,
            fire_penetration: 0.0,
            mana_leech: 0.0,
            cast_speed: 0.0,
            threat: 100.0,
            parry: 0.0,
            mayhem: 0.0,
            pierce: 0.0,
            curse: 0.0,
            ripple: 0.0,
            bloom: 0.0,
            blaze: 0.0,
            weaken: 0.0,
            fury: 0.0,
            food_haste: 0.0,
            drink_concentration: 0.0,
            damage_taken: 0.0,
            attack_speed: 0.0,
            armor_damage_ratio: 0.0,
            hp_drain_ratio: 0.0,
            primary_training: String::new(),
            focus_training: String::new(),
            stamina_experience: 0.0,
            intelligence_experience: 0.0,
            attack_experience: 0.0,
            defense_experience: 0.0,
            melee_experience: 0.0,
            ranged_experience: 0.0,
            magic_experience: 0.0,
            retaliation: 0.0,
            max_hitpoints_ratio: 0.0,
            max_manapoints_ratio: 0.0,
            ability_haste: 0.0,
            tenacity: 0.0,
        }
    }
}

// -- CombatDetails ------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombatDetails {
    pub stamina_level: f64,
    pub intelligence_level: f64,
    pub attack_level: f64,
    pub melee_level: f64,
    pub defense_level: f64,
    pub ranged_level: f64,
    pub magic_level: f64,
    pub max_hitpoints: i64,
    pub current_hitpoints: i64,
    pub max_manapoints: i64,
    pub current_manapoints: i64,
    pub stab_accuracy_rating: f64,
    pub slash_accuracy_rating: f64,
    pub smash_accuracy_rating: f64,
    pub ranged_accuracy_rating: f64,
    pub magic_accuracy_rating: f64,
    pub stab_max_damage: f64,
    pub slash_max_damage: f64,
    pub smash_max_damage: f64,
    pub ranged_max_damage: f64,
    pub magic_max_damage: f64,
    pub stab_evasion_rating: f64,
    pub slash_evasion_rating: f64,
    pub smash_evasion_rating: f64,
    pub ranged_evasion_rating: f64,
    pub magic_evasion_rating: f64,
    pub defensive_max_damage: f64,
    pub total_armor: f64,
    pub total_water_resistance: f64,
    pub total_nature_resistance: f64,
    pub total_fire_resistance: f64,
    pub ability_haste: f64,
    pub tenacity: f64,
    pub total_threat: f64,
    pub combat_stats: CombatStats,
}

impl Default for CombatDetails {
    fn default() -> Self {
        CombatDetails {
            stamina_level: 1.0,
            intelligence_level: 1.0,
            attack_level: 1.0,
            melee_level: 1.0,
            defense_level: 1.0,
            ranged_level: 1.0,
            magic_level: 1.0,
            max_hitpoints: 110,
            current_hitpoints: 110,
            max_manapoints: 110,
            current_manapoints: 110,
            stab_accuracy_rating: 11.0,
            slash_accuracy_rating: 11.0,
            smash_accuracy_rating: 11.0,
            ranged_accuracy_rating: 11.0,
            magic_accuracy_rating: 11.0,
            stab_max_damage: 11.0,
            slash_max_damage: 11.0,
            smash_max_damage: 11.0,
            ranged_max_damage: 11.0,
            magic_max_damage: 11.0,
            stab_evasion_rating: 11.0,
            slash_evasion_rating: 11.0,
            smash_evasion_rating: 11.0,
            ranged_evasion_rating: 11.0,
            magic_evasion_rating: 11.0,
            defensive_max_damage: 0.0,
            total_armor: 0.2,
            total_water_resistance: 0.4,
            total_nature_resistance: 0.4,
            total_fire_resistance: 0.4,
            ability_haste: 0.0,
            tenacity: 0.0,
            total_threat: 100.0,
            combat_stats: CombatStats::default(),
        }
    }
}

// -- BoostResult (from getBuffBoost) -----------------------------------------

pub struct BoostResult {
    pub ratio_boost: f64,
    pub flat_boost: f64,
}

// -- BuffInstance (per-source buff application, for strongest-active tracking) -

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BuffInstance {
    buff: Buff,
    /// Index (in the simulator's unit list) of the unit that applied this
    /// buff. Re-applying from the same source replaces its own instance.
    source: usize,
}

// -- CombatUnit ---------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombatUnit {
    pub hrid: String,
    pub is_player: bool,
    pub is_stunned: bool,
    pub stun_expire_time: Option<i64>,
    pub is_blinded: bool,
    pub blind_expire_time: Option<i64>,
    pub is_silenced: bool,
    pub silence_expire_time: Option<i64>,
    pub is_out_of_mana: bool,

    // Base levels
    pub stamina_level: f64,
    pub intelligence_level: f64,
    pub attack_level: f64,
    pub melee_level: f64,
    pub defense_level: f64,
    pub ranged_level: f64,
    pub magic_level: f64,

    pub experience: f64,
    pub experience_rate: f64,
    pub enrage_time: i64,

    pub abilities: [Option<Ability>; 5],
    pub food: [Option<Consumable>; 3],
    pub drinks: [Option<Consumable>; 3],
    pub house_rooms: Vec<HouseRoom>,

    /// Effective (strongest-active) buff per unique_hrid — derived from
    /// `buff_instances` on every add/remove/expiry. This is what
    /// get_buff_boost(s) reads.
    pub combat_buffs: HashMap<String, Buff>,
    /// Per-source buff instances, keyed by unique_hrid. Multiple units can
    /// apply the same unique_hrid buff (e.g. several players with the same
    /// aura ability at different levels); each keeps its own timer, and
    /// `combat_buffs` always reflects the strongest instance still active.
    /// When the strongest expires, the next-strongest surviving instance
    /// takes over automatically.
    buff_instances: HashMap<String, Vec<BuffInstance>>,
    /// permanent buffs indexed by type_hrid (aggregated)
    pub permanent_buffs: HashMap<String, Buff>,
    pub zone_buffs: Vec<Buff>,
    pub extra_buffs: Vec<Buff>,

    pub combat_details: CombatDetails,
    pub debuff_on_level_gap: f64,

    /// Track mana spent per ability hrid
    pub ability_mana_costs: HashMap<String, i64>,

    /// Equipment slot map (for use by Player subtype)
    pub equipment: HashMap<String, Option<crate::combatsimulator::equipment::Equipment>>,

    /// Monster-specific fields (zero/empty for players)
    pub drop_table: Vec<crate::combatsimulator::drops::Drops>,
    pub rare_drop_table: Vec<crate::combatsimulator::drops::Drops>,
    pub monster_difficulty_tier: i32,
    pub monster_room_level: i32,
    /// Party size the encounter was generated for. Guild monsters (isGuildMonster)
    /// scale their max HP by +1% per player in the party.
    pub monster_num_players: i32,

    /// Snapshot of combat stats after equipment is applied but before any buffs.
    /// Restored at the start of every update_combat_details_base call so buff
    /// additions don't stack across multiple recalculations.
    pub base_combat_stats: CombatStats,
}

impl CombatUnit {
    pub fn new_base() -> Self {
        CombatUnit {
            hrid: String::new(),
            is_player: false,
            is_stunned: false,
            stun_expire_time: None,
            is_blinded: false,
            blind_expire_time: None,
            is_silenced: false,
            silence_expire_time: None,
            is_out_of_mana: false,
            stamina_level: 1.0,
            intelligence_level: 1.0,
            attack_level: 1.0,
            melee_level: 1.0,
            defense_level: 1.0,
            ranged_level: 1.0,
            magic_level: 1.0,
            experience: 0.0,
            experience_rate: 0.0,
            enrage_time: 0,
            abilities: [None, None, None, None, None],
            food: [None, None, None],
            drinks: [None, None, None],
            house_rooms: Vec::new(),
            combat_buffs: HashMap::new(),
            buff_instances: HashMap::new(),
            permanent_buffs: HashMap::new(),
            zone_buffs: Vec::new(),
            extra_buffs: Vec::new(),
            combat_details: CombatDetails::default(),
            debuff_on_level_gap: 0.0,
            ability_mana_costs: HashMap::new(),
            equipment: HashMap::new(),
            drop_table: Vec::new(),
            rare_drop_table: Vec::new(),
            monster_difficulty_tier: 0,
            monster_room_level: 0,
            monster_num_players: 0,
            base_combat_stats: CombatStats::default(),
        }
    }

    // -- Buff boosts ----------------------------------------------------------

    pub fn get_buff_boosts(&self, type_hrid: &str) -> Vec<BoostResult> {
        self.combat_buffs.values()
            .filter(|b| b.type_hrid == type_hrid)
            .map(|b| BoostResult { ratio_boost: b.ratio_boost, flat_boost: b.flat_boost })
            .collect()
    }

    pub fn get_buff_boost(&self, type_hrid: &str) -> BoostResult {
        let boosts = self.get_buff_boosts(type_hrid);
        let mut ratio = 0.0;
        let mut flat = 0.0;
        for b in &boosts {
            ratio += b.ratio_boost;
            flat += b.flat_boost;
        }
        BoostResult { ratio_boost: ratio, flat_boost: flat }
    }

    pub fn has_buff_starting_with(&self, prefix: &str) -> bool {
        self.combat_buffs.keys().any(|k| k.starts_with(prefix))
    }

    pub fn get_buff_with_prefix(&self, prefix: &str) -> Option<&Buff> {
        self.combat_buffs.iter()
            .find(|(k, _)| k.starts_with(prefix))
            .map(|(_, v)| v)
    }

    // -- Add / remove buffs ---------------------------------------------------
    //
    // Multiple units can apply the same unique_hrid buff (e.g. several players
    // with the same aura ability at different levels, or several monsters
    // inflicting the same debuff). Each application is tracked as its own
    // timed instance, keyed by the applying unit's index ("source"); the
    // effective value exposed via `combat_buffs` (and thus get_buff_boost) is
    // always the strongest still-active instance. When the strongest expires,
    // the next-strongest surviving instance takes over automatically.

    fn instance_active(inst: &BuffInstance, current_time: i64) -> bool {
        inst.buff.duration <= 0 || inst.buff.start_time + inst.buff.duration > current_time
    }

    /// Larger |ratio_boost| wins, then larger |flat_boost|, then later expiry
    /// (never-expiring instances, duration <= 0, treated as +infinity). Magnitude
    /// (not signed value) matters because debuffs carry negative boosts and
    /// "stronger" means larger effect either way.
    fn stronger_instance<'a>(a: &'a BuffInstance, b: &'a BuffInstance) -> &'a BuffInstance {
        let ar = a.buff.ratio_boost.abs();
        let br = b.buff.ratio_boost.abs();
        if ar != br { return if ar > br { a } else { b }; }
        let af = a.buff.flat_boost.abs();
        let bf = b.buff.flat_boost.abs();
        if af != bf { return if af > bf { a } else { b }; }
        let ae = if a.buff.duration <= 0 { i64::MAX } else { a.buff.start_time + a.buff.duration };
        let be = if b.buff.duration <= 0 { i64::MAX } else { b.buff.start_time + b.buff.duration };
        if ae >= be { a } else { b }
    }

    fn effective_instance(instances: &[BuffInstance]) -> Option<Buff> {
        instances.iter()
            .fold(None, |best: Option<&BuffInstance>, inst| Some(match best {
                Some(b) => Self::stronger_instance(b, inst),
                None => inst,
            }))
            .map(|inst| inst.buff.clone())
    }

    /// Recompute the effective view for `unique_hrid` from its instance list
    /// and write it into `combat_buffs` (removing the key if the list is
    /// empty). Returns whether the effective ratio/flat boost changed.
    fn commit_instances(&mut self, unique_hrid: &str) -> bool {
        let prev = self.combat_buffs.get(unique_hrid).cloned();
        let effective = self.buff_instances.get(unique_hrid)
            .and_then(|instances| Self::effective_instance(instances));

        match effective {
            None => {
                self.buff_instances.remove(unique_hrid);
                self.combat_buffs.remove(unique_hrid).is_some()
            }
            Some(eff) => {
                let changed = prev
                    .map(|p| p.ratio_boost != eff.ratio_boost || p.flat_boost != eff.flat_boost)
                    .unwrap_or(true);
                self.combat_buffs.insert(unique_hrid.to_string(), eff);
                changed
            }
        }
    }

    /// Apply one buff instance from `source` and return whether the effective
    /// (strongest-active) view for its unique_hrid changed.
    fn apply_buff_instance(&mut self, mut buff: Buff, current_time: i64, source: usize) -> bool {
        buff.start_time = current_time;
        let hrid = buff.unique_hrid.clone();
        let entry = self.buff_instances.entry(hrid.clone()).or_default();
        // Same-source re-apply replaces its own instance (buff refresh); drop
        // any instance that has since expired.
        entry.retain(|inst| inst.source != source && Self::instance_active(inst, current_time));
        entry.push(BuffInstance { buff, source });
        self.commit_instances(&hrid)
    }

    pub fn add_buff(&mut self, buff: Buff, current_time: i64, source: usize) -> bool {
        let changed = self.apply_buff_instance(buff, current_time, source);
        if changed {
            self.update_combat_details();
        }
        changed
    }

    pub fn add_buffs(&mut self, buffs: Vec<Buff>, current_time: i64, source: usize) {
        let mut needs_update = false;
        for buff in buffs {
            if self.apply_buff_instance(buff, current_time, source) {
                needs_update = true;
            }
        }
        if needs_update {
            self.update_combat_details();
        }
    }

    /// Remove `source`'s instance of the given unique_hrid buff (if any). The
    /// next-strongest surviving instance from another source, if any, takes
    /// over as the effective value.
    pub fn remove_buff_by_unique(&mut self, unique_hrid: &str, source: usize) {
        let Some(entry) = self.buff_instances.get_mut(unique_hrid) else { return };
        let before = entry.len();
        entry.retain(|inst| inst.source != source);
        if entry.len() == before { return; }
        if self.commit_instances(unique_hrid) {
            self.update_combat_details();
        }
    }

    pub fn remove_buffs(&mut self, buffs: &[Buff], source: usize) {
        let mut changed = false;
        for buff in buffs {
            let Some(entry) = self.buff_instances.get_mut(&buff.unique_hrid) else { continue };
            let before = entry.len();
            entry.retain(|inst| inst.source != source);
            if entry.len() != before && self.commit_instances(&buff.unique_hrid) {
                changed = true;
            }
        }
        if changed {
            self.update_combat_details();
        }
    }

    pub fn add_permanent_buff(&mut self, buff: &Buff) {
        let entry = self.permanent_buffs.entry(buff.type_hrid.clone()).or_insert_with(|| Buff {
            unique_hrid: buff.unique_hrid.clone(),
            type_hrid: buff.type_hrid.clone(),
            flat_boost: 0.0,
            ratio_boost: 0.0,
            duration: buff.duration,
            multiplier_for_skill_hrid: String::new(),
            multiplier_per_skill_level: 0.0,
            start_time: 0,
        });
        entry.flat_boost += buff.flat_boost;
        entry.ratio_boost += buff.ratio_boost;
    }

    pub fn generate_permanent_buffs(&mut self) {
        let rooms = self.house_rooms.clone();
        for room in &rooms {
            for buff in &room.buffs {
                self.add_permanent_buff(buff);
            }
        }
        let zone_buffs = self.zone_buffs.clone();
        for buff in &zone_buffs {
            self.add_permanent_buff(buff);
        }
        let extra_buffs = self.extra_buffs.clone();
        for buff in &extra_buffs {
            self.add_permanent_buff(buff);
        }
    }

    pub fn remove_expired_buffs(&mut self, current_time: i64) {
        let hrids: Vec<String> = self.buff_instances.keys().cloned().collect();
        for hrid in hrids {
            let entry = self.buff_instances.get_mut(&hrid).unwrap();
            entry.retain(|inst| Self::instance_active(inst, current_time));
            self.commit_instances(&hrid);
        }
        self.update_combat_details();
    }

    pub fn clear_buffs(&mut self) {
        self.buff_instances.clear();
        self.combat_buffs = self.permanent_buffs.values().cloned()
            .map(|b| (b.unique_hrid.clone(), b))
            .collect();
        self.update_combat_details();
    }

    pub fn clear_ccs(&mut self) {
        self.is_stunned = false;
        self.stun_expire_time = None;
        self.is_silenced = false;
        self.silence_expire_time = None;
        self.is_blinded = false;
        self.blind_expire_time = None;
        self.combat_details.combat_stats.damage_taken = 0.0;
    }

    // -- HP / MP --------------------------------------------------------------

    pub fn add_hitpoints(&mut self, hp: i64) -> i64 {
        if self.combat_details.current_hitpoints >= self.combat_details.max_hitpoints {
            return 0;
        }
        let new_hp = (self.combat_details.current_hitpoints + hp).min(self.combat_details.max_hitpoints);
        let added = new_hp - self.combat_details.current_hitpoints;
        self.combat_details.current_hitpoints = new_hp;
        added
    }

    pub fn add_manapoints(&mut self, mp: i64) -> i64 {
        if self.combat_details.current_manapoints >= self.combat_details.max_manapoints {
            return 0;
        }
        let new_mp = (self.combat_details.current_manapoints + mp).min(self.combat_details.max_manapoints);
        let added = new_mp - self.combat_details.current_manapoints;
        self.combat_details.current_manapoints = new_mp;
        added
    }

    // -- Reset ----------------------------------------------------------------

    pub fn reset(&mut self, current_time: i64) {
        self.clear_ccs();
        if current_time == 0 || !self.is_player {
            self.clear_buffs();
            self.reset_cooldowns(current_time);
        } else {
            self.remove_expired_buffs(current_time);
        }
        self.combat_details.current_hitpoints = self.combat_details.max_hitpoints;
        self.combat_details.current_manapoints = self.combat_details.max_manapoints;
    }

    pub fn reset_cooldowns(&mut self, current_time: i64) {
        for food in self.food.iter_mut().flatten() {
            food.last_used = i64::MIN;
        }
        for drink in self.drinks.iter_mut().flatten() {
            drink.last_used = i64::MIN;
        }

        let haste = self.combat_details.combat_stats.ability_haste;
        let is_player = self.is_player;

        for ability_opt in self.abilities.iter_mut() {
            if let Some(ability) = ability_opt {
                if is_player {
                    ability.last_used = i64::MIN;
                } else {
                    let mut cd = ability.cooldown_duration as f64;
                    if haste > 0.0 {
                        cd = cd * 100.0 / (100.0 + haste);
                    }
                    let cd_i = cd as i64;
                    ability.last_used = current_time - (cd_i / 2);
                }
            }
        }
    }

    // -- Update combat details (recalculate derived stats) --------------------
    /// This is the Rust equivalent of CombatUnit.updateCombatDetails() in JS.
    /// Player and Monster each override this; this base version handles shared logic.
    pub fn update_combat_details_base(&mut self) {
        // Restore equipment-baseline stats before re-applying buff deltas.
        // Without this, every buff add/remove call stacks += on top of previous totals.
        self.reset_stats_to_base();

        if self.is_player {
            if self.combat_details.combat_stats.hp_regen_per10 == 0.0 {
                self.combat_details.combat_stats.hp_regen_per10 = 0.01;
            } else {
                self.combat_details.combat_stats.hp_regen_per10 = 0.01 + self.combat_details.combat_stats.hp_regen_per10;
            }
            if self.combat_details.combat_stats.mp_regen_per10 == 0.0 {
                self.combat_details.combat_stats.mp_regen_per10 = 0.01;
            } else {
                self.combat_details.combat_stats.mp_regen_per10 = 0.01 + self.combat_details.combat_stats.mp_regen_per10;
            }
        }

        // Level boosts from buffs
        for stat in &["stamina", "intelligence", "attack", "melee", "defense", "ranged", "magic"] {
            let base = self.base_level(stat);
            let type_hrid = format!("/buff_types/{}_level", stat);
            let boosts = self.get_buff_boosts(&type_hrid);
            let mut boosted = base;
            for b in &boosts {
                boosted += base * b.ratio_boost;
                boosted += b.flat_boost;
            }
            self.set_cd_level(stat, boosted);
        }

        self.combat_details.combat_stats.max_hitpoints_ratio += self.get_buff_boost("/buff_types/max_hitpoints").ratio_boost;
        self.combat_details.combat_stats.max_manapoints_ratio += self.get_buff_boost("/buff_types/max_manapoints").ratio_boost;

        self.combat_details.max_hitpoints = ((10.0 * (10.0 + self.combat_details.stamina_level)
            + self.combat_details.combat_stats.max_hitpoints)
            * (1.0 + self.combat_details.combat_stats.max_hitpoints_ratio))
            .floor() as i64;

        self.combat_details.max_manapoints = ((10.0 * (10.0 + self.combat_details.intelligence_level)
            + self.combat_details.combat_stats.max_manapoints)
            * (1.0 + self.combat_details.combat_stats.max_manapoints_ratio))
            .floor() as i64;

        let fury_acc_boost = self.get_buff_boost("/buff_types/fury_accuracy").ratio_boost;
        let fury_dmg_boost = self.get_buff_boost("/buff_types/fury_damage").ratio_boost;
        let acc_boost = self.get_buff_boost("/buff_types/accuracy").ratio_boost;
        let dmg_boost = self.get_buff_boost("/buff_types/damage").ratio_boost;

        for style in &["stab", "slash", "smash"] {
            let acc = (10.0 + self.combat_details.attack_level)
                * (1.0 + self.style_stat(style, "accuracy"))
                * (1.0 + acc_boost)
                * (1.0 + fury_acc_boost);
            let dmg = (10.0 + self.combat_details.melee_level)
                * (1.0 + self.style_stat(style, "damage"))
                * (1.0 + dmg_boost)
                * (1.0 + fury_dmg_boost);
            let base_evasion = (10.0 + self.combat_details.defense_level)
                * (1.0 + self.style_stat(style, "evasion"));
            let evasion_boosts = self.get_buff_boosts("/buff_types/evasion");
            let mut ev = base_evasion;
            for b in &evasion_boosts {
                ev += b.flat_boost;
                ev += base_evasion * b.ratio_boost;
            }
            self.set_style_acc(style, acc);
            self.set_style_dmg(style, dmg);
            self.set_style_ev(style, ev);
        }

        self.combat_details.defensive_max_damage =
            (10.0 + self.combat_details.defense_level)
            * (1.0 + self.combat_details.combat_stats.defensive_damage)
            * (1.0 + dmg_boost)
            * (1.0 + fury_dmg_boost);

        // bulwark two-hand check
        let has_bulwark = self.equipment.get("/equipment_types/two_hand")
            .and_then(|e| e.as_ref())
            .map(|e| e.hrid.contains("bulwark"))
            .unwrap_or(false);
        if has_bulwark {
            self.combat_details.smash_max_damage += self.combat_details.defensive_max_damage;
        }

        let ranged_acc = (10.0 + self.combat_details.attack_level)
            * (1.0 + self.combat_details.combat_stats.ranged_accuracy)
            * (1.0 + acc_boost)
            * (1.0 + fury_acc_boost);
        self.combat_details.ranged_accuracy_rating = ranged_acc;
        let ranged_dmg = (10.0 + self.combat_details.ranged_level)
            * (1.0 + self.combat_details.combat_stats.ranged_damage)
            * (1.0 + dmg_boost)
            * (1.0 + fury_dmg_boost);
        self.combat_details.ranged_max_damage = ranged_dmg;

        let base_ranged_ev = (10.0 + self.combat_details.defense_level)
            * (1.0 + self.combat_details.combat_stats.ranged_evasion);
        let evasion_boosts = self.get_buff_boosts("/buff_types/evasion");
        let mut rev = base_ranged_ev;
        for b in &evasion_boosts {
            rev += b.flat_boost;
            rev += base_ranged_ev * b.ratio_boost;
        }
        self.combat_details.ranged_evasion_rating = rev;

        self.combat_details.combat_stats.damage_taken = self.get_buff_boost("/buff_types/damage_taken").flat_boost;

        let magic_acc = (10.0 + self.combat_details.attack_level)
            * (1.0 + self.combat_details.combat_stats.magic_accuracy)
            * (1.0 + acc_boost)
            * (1.0 + fury_acc_boost);
        self.combat_details.magic_accuracy_rating = magic_acc;
        let magic_dmg = (10.0 + self.combat_details.magic_level)
            * (1.0 + self.combat_details.combat_stats.magic_damage)
            * (1.0 + dmg_boost)
            * (1.0 + fury_dmg_boost);
        self.combat_details.magic_max_damage = magic_dmg;

        let base_magic_ev = (10.0 + self.combat_details.defense_level)
            * (1.0 + self.combat_details.combat_stats.magic_evasion);
        let evasion_boosts = self.get_buff_boosts("/buff_types/evasion");
        let mut mev = base_magic_ev;
        for b in &evasion_boosts {
            mev += b.flat_boost;
            mev += base_magic_ev * b.ratio_boost;
        }
        self.combat_details.magic_evasion_rating = mev;

        // Amplifies from buffs
        self.combat_details.combat_stats.physical_amplify += self.get_buff_boost("/buff_types/physical_amplify").flat_boost;
        self.combat_details.combat_stats.water_amplify += self.get_buff_boost("/buff_types/water_amplify").flat_boost;
        self.combat_details.combat_stats.nature_amplify += self.get_buff_boost("/buff_types/nature_amplify").flat_boost;
        self.combat_details.combat_stats.fire_amplify += self.get_buff_boost("/buff_types/fire_amplify").flat_boost;
        self.combat_details.combat_stats.healing_amplify += self.get_buff_boost("/buff_types/healing_amplify").flat_boost;

        // Attack interval
        self.combat_details.combat_stats.attack_interval /= 1.0 + (self.combat_details.attack_level / 2000.0);
        let base_attack_speed = self.combat_details.combat_stats.attack_speed;
        self.combat_details.combat_stats.attack_interval /= 1.0 + base_attack_speed;
        let attack_speed_ratio: f64 = self.get_buff_boosts("/buff_types/attack_speed")
            .iter().map(|b| b.ratio_boost).sum();
        self.combat_details.combat_stats.attack_interval /= 1.0 + attack_speed_ratio;

        // Armor/resistances
        let base_armor = 0.2 * self.combat_details.defense_level + self.combat_details.combat_stats.armor;
        self.combat_details.total_armor = base_armor;
        for b in self.get_buff_boosts("/buff_types/armor") {
            self.combat_details.total_armor += b.flat_boost;
            self.combat_details.total_armor += base_armor * b.ratio_boost;
        }

        let base_water = 0.2 * self.combat_details.defense_level + self.combat_details.combat_stats.water_resistance;
        self.combat_details.total_water_resistance = base_water;
        for b in self.get_buff_boosts("/buff_types/water_resistance") {
            self.combat_details.total_water_resistance += b.flat_boost;
            self.combat_details.total_water_resistance += base_water * b.ratio_boost;
        }

        let base_nature = 0.2 * self.combat_details.defense_level + self.combat_details.combat_stats.nature_resistance;
        self.combat_details.total_nature_resistance = base_nature;
        for b in self.get_buff_boosts("/buff_types/nature_resistance") {
            self.combat_details.total_nature_resistance += b.flat_boost;
            self.combat_details.total_nature_resistance += base_nature * b.ratio_boost;
        }

        let base_fire = 0.2 * self.combat_details.defense_level + self.combat_details.combat_stats.fire_resistance;
        self.combat_details.total_fire_resistance = base_fire;
        for b in self.get_buff_boosts("/buff_types/fire_resistance") {
            self.combat_details.total_fire_resistance += b.flat_boost;
            self.combat_details.total_fire_resistance += base_fire * b.ratio_boost;
        }

        // Regen boosts
        let hp_regen_boost = self.get_buff_boost("/buff_types/hp_regen");
        self.combat_details.combat_stats.hp_regen_per10 += self.combat_details.combat_stats.hp_regen_per10 * hp_regen_boost.ratio_boost;
        self.combat_details.combat_stats.hp_regen_per10 += hp_regen_boost.flat_boost;

        let mp_regen_boost = self.get_buff_boost("/buff_types/mp_regen");
        self.combat_details.combat_stats.mp_regen_per10 += self.combat_details.combat_stats.mp_regen_per10 * mp_regen_boost.ratio_boost;
        self.combat_details.combat_stats.mp_regen_per10 += mp_regen_boost.flat_boost;

        // Other stats from buffs
        self.combat_details.combat_stats.life_steal += self.get_buff_boost("/buff_types/life_steal").flat_boost;
        self.combat_details.combat_stats.physical_thorns += self.get_buff_boost("/buff_types/physical_thorns").flat_boost;
        self.combat_details.combat_stats.elemental_thorns += self.get_buff_boost("/buff_types/elemental_thorns").flat_boost;
        self.combat_details.combat_stats.combat_experience += self.get_buff_boost("/buff_types/wisdom").flat_boost;
        self.combat_details.combat_stats.critical_rate += self.get_buff_boost("/buff_types/critical_rate").flat_boost;
        self.combat_details.combat_stats.critical_damage += self.get_buff_boost("/buff_types/critical_damage").flat_boost;
        self.combat_details.combat_stats.cast_speed += self.get_buff_boost("/buff_types/cast_speed").flat_boost;
        self.combat_details.combat_stats.cast_speed += self.combat_details.attack_level / 2000.0;

        // Drop stats
        let drop_rate_boost = self.get_buff_boost("/buff_types/combat_drop_rate");
        self.combat_details.combat_stats.combat_drop_rate += (1.0 + self.combat_details.combat_stats.combat_drop_rate) * drop_rate_boost.ratio_boost;
        self.combat_details.combat_stats.combat_drop_rate += drop_rate_boost.flat_boost;

        let rare_find_boost = self.get_buff_boost("/buff_types/rare_find");
        self.combat_details.combat_stats.combat_rare_find += (1.0 + self.combat_details.combat_stats.combat_rare_find) * rare_find_boost.ratio_boost;
        self.combat_details.combat_stats.combat_rare_find += rare_find_boost.flat_boost;

        let drop_qty_boost = self.get_buff_boost("/buff_types/combat_drop_quantity");
        self.combat_details.combat_stats.combat_drop_quantity += (1.0 + self.combat_details.combat_stats.combat_drop_quantity) * drop_qty_boost.ratio_boost;
        self.combat_details.combat_stats.combat_drop_quantity += drop_qty_boost.flat_boost;

        // Threat
        let base_threat = 100.0 + self.combat_details.combat_stats.threat;
        self.combat_details.total_threat = base_threat;
        let threat_boost = self.get_buff_boost("/buff_types/threat");
        // Always start from base_threat (mirrors JS: else branch sets threat=base, if branch adds ratio on top)
        self.combat_details.combat_stats.threat = base_threat;
        if threat_boost.ratio_boost != 0.0 {
            self.combat_details.combat_stats.threat += base_threat * threat_boost.ratio_boost;
        }
        self.combat_details.combat_stats.threat += threat_boost.flat_boost;

        self.combat_details.combat_stats.retaliation += self.get_buff_boost("/buff_types/retaliation").flat_boost;
        self.combat_details.combat_stats.tenacity += self.get_buff_boost("/buff_types/tenacity").flat_boost;
        self.combat_details.ability_haste = self.combat_details.combat_stats.ability_haste;
        self.combat_details.tenacity = self.combat_details.combat_stats.tenacity;
    }

    /// Dispatches to the monster-specific recompute (which re-applies the
    /// guild-trial participant scaling on every call, not just at spawn) for
    /// monsters, or the shared base recompute for players (players go through
    /// PlayerExt::player_update_combat_details for their own initial build,
    /// which already ends with update_combat_details_base()).
    ///
    /// This matters because ANY buff change (an aura landing, a curse tick,
    /// anything) calls this generic path. Before this fix, that always ran
    /// update_combat_details_base() alone, which restores combat_stats from
    /// base_combat_stats — a snapshot taken BEFORE monster_update_combat_details
    /// applies the +2%/+2%/+2 participant bonus. So the first buff event of
    /// the fight silently erased a guild monster's participant scaling for
    /// the rest of the encounter.
    pub fn update_combat_details(&mut self) {
        let is_real_monster = !self.is_player
            && crate::combatsimulator::data::combat_monster_detail_map().contains_key(&self.hrid);
        if is_real_monster {
            crate::combatsimulator::monster::monster_update_combat_details(self);
        } else {
            self.update_combat_details_base();
        }
    }

    // -- Helpers --------------------------------------------------------------

    fn base_level(&self, stat: &str) -> f64 {
        match stat {
            "stamina" => self.stamina_level,
            "intelligence" => self.intelligence_level,
            "attack" => self.attack_level,
            "melee" => self.melee_level,
            "defense" => self.defense_level,
            "ranged" => self.ranged_level,
            "magic" => self.magic_level,
            _ => 1.0,
        }
    }

    fn set_cd_level(&mut self, stat: &str, val: f64) {
        match stat {
            "stamina" => self.combat_details.stamina_level = val,
            "intelligence" => self.combat_details.intelligence_level = val,
            "attack" => self.combat_details.attack_level = val,
            "melee" => self.combat_details.melee_level = val,
            "defense" => self.combat_details.defense_level = val,
            "ranged" => self.combat_details.ranged_level = val,
            "magic" => self.combat_details.magic_level = val,
            _ => {}
        }
    }

    fn style_stat(&self, style: &str, kind: &str) -> f64 {
        match (style, kind) {
            ("stab", "accuracy") => self.combat_details.combat_stats.stab_accuracy,
            ("slash", "accuracy") => self.combat_details.combat_stats.slash_accuracy,
            ("smash", "accuracy") => self.combat_details.combat_stats.smash_accuracy,
            ("stab", "damage") => self.combat_details.combat_stats.stab_damage,
            ("slash", "damage") => self.combat_details.combat_stats.slash_damage,
            ("smash", "damage") => self.combat_details.combat_stats.smash_damage,
            ("stab", "evasion") => self.combat_details.combat_stats.stab_evasion,
            ("slash", "evasion") => self.combat_details.combat_stats.slash_evasion,
            ("smash", "evasion") => self.combat_details.combat_stats.smash_evasion,
            _ => 0.0,
        }
    }

    fn set_style_acc(&mut self, style: &str, val: f64) {
        match style {
            "stab" => self.combat_details.stab_accuracy_rating = val,
            "slash" => self.combat_details.slash_accuracy_rating = val,
            "smash" => self.combat_details.smash_accuracy_rating = val,
            _ => {}
        }
    }

    fn set_style_dmg(&mut self, style: &str, val: f64) {
        match style {
            "stab" => self.combat_details.stab_max_damage = val,
            "slash" => self.combat_details.slash_max_damage = val,
            "smash" => self.combat_details.smash_max_damage = val,
            _ => {}
        }
    }

    fn set_style_ev(&mut self, style: &str, val: f64) {
        match style {
            "stab" => self.combat_details.stab_evasion_rating = val,
            "slash" => self.combat_details.slash_evasion_rating = val,
            "smash" => self.combat_details.smash_evasion_rating = val,
            _ => {}
        }
    }
}

impl CombatUnit {
    /// Reset all stats that update_combat_details_base accumulates with +=
    /// back to the values set by equipment (stored in base_combat_stats).
    /// Must be called at the start of update_combat_details_base.
    pub fn reset_stats_to_base(&mut self) {
        let b = &self.base_combat_stats;
        let s = &mut self.combat_details.combat_stats;
        s.physical_amplify    = b.physical_amplify;
        s.water_amplify       = b.water_amplify;
        s.nature_amplify      = b.nature_amplify;
        s.fire_amplify        = b.fire_amplify;
        s.healing_amplify     = b.healing_amplify;
        s.life_steal          = b.life_steal;
        s.physical_thorns     = b.physical_thorns;
        s.elemental_thorns    = b.elemental_thorns;
        s.combat_experience   = b.combat_experience;
        s.critical_rate       = b.critical_rate;
        s.critical_damage     = b.critical_damage;
        s.cast_speed          = b.cast_speed;
        s.combat_drop_rate    = b.combat_drop_rate;
        s.combat_rare_find    = b.combat_rare_find;
        s.combat_drop_quantity = b.combat_drop_quantity;
        s.hp_regen_per10      = b.hp_regen_per10;
        s.mp_regen_per10      = b.mp_regen_per10;
        s.max_hitpoints_ratio  = b.max_hitpoints_ratio;
        s.max_manapoints_ratio = b.max_manapoints_ratio;
        s.retaliation         = b.retaliation;
        s.tenacity            = b.tenacity;
        s.ability_haste       = b.ability_haste;
        s.threat              = b.threat;
        s.attack_interval     = b.attack_interval;
        s.attack_speed        = b.attack_speed;
        s.damage_taken        = 0.0; // always from buffs only
    }
}
