use rand::Rng;
use crate::combatsimulator::{ability::AbilityEffect, combat_unit::CombatUnit};

pub struct AttackResult {
    pub damage_done: i64,
    pub did_hit: bool,
    pub thorn_damage_done: i64,
    pub thorn_type: String,
    pub retaliation_damage_done: i64,
    pub life_steal_heal: i64,
    pub hp_drain: i64,
    pub mana_leech_mana: i64,
    pub is_crit: bool,
    /// Portion of damage_done attributable to the target's damage_taken debuff
    /// (curse, fracturing_impact, etc.). = damage_done * dt / (1 + dt)
    pub debuff_damage: i64,
    /// Per-buff resistance debuff attribution: (unique_hrid, damage_contribution)
    /// Each entry = damage that wouldn't have happened without that resistance debuff.
    pub resist_debuff_damage: Vec<(String, i64)>,
    /// damage_done / thorn_damage_done / retaliation_damage_done before the
    /// target's armor/resistance mitigation ratio (and the current-HP overkill
    /// clamp) is applied - i.e. the raw hit that landed, before defenses.
    pub raw_damage: i64,
    pub raw_thorn_damage: i64,
    pub raw_retaliation_damage: i64,
}

pub struct CombatUtilities;

impl CombatUtilities {
    pub fn get_target(units: &[CombatUnit]) -> Option<usize> {
        units.iter().position(|u| u.combat_details.current_hitpoints > 0)
    }

    pub fn random_int(min: f64, max: f64) -> i64 {
        let (min, max) = if max < min { (max, min) } else { (min, max) };
        let mut rng = rand::thread_rng();

        let min_ceil = min.ceil();
        let max_floor = max.floor();

        if min.floor() == max_floor {
            return (((min + max) / 2.0) + rng.gen::<f64>()).floor() as i64;
        }

        let min_tail = -(min - min_ceil);
        let max_tail = max - max_floor;

        let balanced_weight = 2.0 * min_tail + (max_floor - min_ceil);
        let balanced_average = (max_floor + min_ceil) / 2.0;
        let average = (max + min) / 2.0;
        let extra_tail_weight = if max_floor + 1.0 - average != 0.0 {
            (balanced_weight * (average - balanced_average)) / (max_floor + 1.0 - average)
        } else {
            0.0
        };
        let extra_tail_chance = if extra_tail_weight + balanced_weight != 0.0 {
            (extra_tail_weight / (extra_tail_weight + balanced_weight)).abs()
        } else {
            0.0
        };

        if rng.gen::<f64>() < extra_tail_chance {
            return if max_tail > min_tail {
                (max_floor + 1.0).floor() as i64
            } else {
                (min_ceil - 1.0).floor() as i64
            };
        }

        if max_tail > min_tail {
            (min + rng.gen::<f64>() * (max_floor + min_tail - min + 1.0)).floor() as i64
        } else {
            ((min_ceil - max_tail) + rng.gen::<f64>() * (max - (min_ceil - max_tail) + 1.0)).floor() as i64
        }
    }

    pub fn process_attack(source: &mut CombatUnit, target: &mut CombatUnit, ability_effect: Option<&AbilityEffect>) -> AttackResult {
        let mut rng = rand::thread_rng();

        let combat_style = ability_effect
            .map(|ae| ae.combat_style_hrid.as_str())
            .unwrap_or(&source.combat_details.combat_stats.combat_style_hrid);
        let combat_style = combat_style.to_string();

        let damage_type = ability_effect
            .map(|ae| ae.damage_type.as_str())
            .unwrap_or(&source.combat_details.combat_stats.damage_type);
        let damage_type = damage_type.to_string();

        let (source_acc, source_auto_max_dmg, target_evasion) = match combat_style.as_str() {
            "/combat_styles/stab" => (source.combat_details.stab_accuracy_rating, source.combat_details.stab_max_damage, target.combat_details.stab_evasion_rating),
            "/combat_styles/slash" => (source.combat_details.slash_accuracy_rating, source.combat_details.slash_max_damage, target.combat_details.slash_evasion_rating),
            "/combat_styles/smash" => (source.combat_details.smash_accuracy_rating, source.combat_details.smash_max_damage, target.combat_details.smash_evasion_rating),
            "/combat_styles/ranged" => (source.combat_details.ranged_accuracy_rating, source.combat_details.ranged_max_damage, target.combat_details.ranged_evasion_rating),
            "/combat_styles/magic" => (source.combat_details.magic_accuracy_rating, source.combat_details.magic_max_damage, target.combat_details.magic_evasion_rating),
            other => panic!("Unknown combat style: {}", other),
        };

        let (source_dmg_mult, source_resistance, source_penetration, target_resistance, target_thorn_power, target_penetration, thorn_type) = match damage_type.as_str() {
            "/damage_types/physical" => (
                1.0 + source.combat_details.combat_stats.physical_amplify,
                source.combat_details.total_armor,
                source.combat_details.combat_stats.armor_penetration,
                target.combat_details.total_armor,
                target.combat_details.combat_stats.physical_thorns,
                target.combat_details.combat_stats.armor_penetration,
                "physicalThorns".to_string(),
            ),
            "/damage_types/water" => (
                1.0 + source.combat_details.combat_stats.water_amplify,
                source.combat_details.total_water_resistance,
                source.combat_details.combat_stats.water_penetration,
                target.combat_details.total_water_resistance,
                target.combat_details.combat_stats.elemental_thorns,
                target.combat_details.combat_stats.water_penetration,
                "elementalThorns".to_string(),
            ),
            "/damage_types/nature" => (
                1.0 + source.combat_details.combat_stats.nature_amplify,
                source.combat_details.total_nature_resistance,
                source.combat_details.combat_stats.nature_penetration,
                target.combat_details.total_nature_resistance,
                target.combat_details.combat_stats.elemental_thorns,
                target.combat_details.combat_stats.nature_penetration,
                "elementalThorns".to_string(),
            ),
            "/damage_types/fire" => (
                1.0 + source.combat_details.combat_stats.fire_amplify,
                source.combat_details.total_fire_resistance,
                source.combat_details.combat_stats.fire_penetration,
                target.combat_details.total_fire_resistance,
                target.combat_details.combat_stats.elemental_thorns,
                target.combat_details.combat_stats.fire_penetration,
                "elementalThorns".to_string(),
            ),
            other => panic!("Unknown damage type: {}", other),
        };

        let mut source_acc_rating = source_acc;
        if let Some(ae) = ability_effect {
            source_acc_rating *= 1.0 + ae.bonus_accuracy_ratio;
        }

        let hit_chance = source_acc_rating.powf(1.4) / (source_acc_rating.powf(1.4) + target_evasion.powf(1.4));

        let mut crit_chance = 0.0;
        if combat_style == "/combat_styles/ranged" {
            crit_chance = 0.3 * hit_chance;
        }
        crit_chance += source.combat_details.combat_stats.critical_rate;
        let bonus_crit_damage = source.combat_details.combat_stats.critical_damage;

        let base_dmg_flat = ability_effect.map(|ae| ae.damage_flat).unwrap_or(0.0);
        let base_dmg_ratio = ability_effect.map(|ae| ae.damage_ratio).unwrap_or(1.0);
        let armor_dmg_ratio_flat = ability_effect
            .map(|ae| ae.armor_damage_ratio * source.combat_details.total_armor)
            .unwrap_or(0.0);

        let mut source_min_dmg = source_dmg_mult * (1.0 + base_dmg_flat + armor_dmg_ratio_flat);
        let mut source_max_dmg = source_dmg_mult * (base_dmg_ratio * source_auto_max_dmg + base_dmg_flat + armor_dmg_ratio_flat);

        let mut is_crit = false;
        if rng.gen::<f64>() < crit_chance {
            source_max_dmg *= 1.0 + bonus_crit_damage;
            source_min_dmg = source_max_dmg;
            is_crit = true;
        }

        let mut damage_roll = Self::random_int(source_min_dmg, source_max_dmg) as f64;
        damage_roll *= 1.0 + source.combat_details.combat_stats.task_damage;
        damage_roll *= 1.0 + target.combat_details.combat_stats.damage_taken;

        if ability_effect.is_none() {
            damage_roll += damage_roll * source.combat_details.combat_stats.auto_attack_damage;
        } else {
            damage_roll *= 1.0 + source.combat_details.combat_stats.ability_damage;
        }

        let mut damage_done: i64 = 0;
        let mut raw_damage: i64 = 0;
        let mut thorn_damage_done: i64 = 0;
        let mut raw_thorn_damage: i64 = 0;
        let mut did_hit = false;

        if rng.gen::<f64>() < hit_chance {
            did_hit = true;
            let mut penetrated = target_resistance;
            if source_penetration > 0.0 && target_resistance > 0.0 {
                penetrated = target_resistance / (1.0 + source_penetration);
            }
            let dmg_taken_ratio = if penetrated < 0.0 {
                (100.0 - penetrated) / 100.0
            } else {
                100.0 / (100.0 + penetrated)
            };
            raw_damage = damage_roll.ceil() as i64;
            let mitigated = (dmg_taken_ratio * damage_roll).ceil() as i64;
            damage_done = mitigated.min(target.combat_details.current_hitpoints);
            target.combat_details.current_hitpoints -= damage_done;
        }

        // Thorns
        if target_thorn_power > 0.0 && target_resistance > -99.0 {
            let mut pen_src_res = source_resistance;
            if source_resistance > 0.0 {
                pen_src_res = source_resistance / (1.0 + target_penetration);
            }
            let src_dmg_taken = if pen_src_res < 0.0 {
                (100.0 - pen_src_res) / 100.0
            } else {
                100.0 / (100.0 + pen_src_res)
            };
            let tgt_task_mult = 1.0 + target.combat_details.combat_stats.task_damage;
            let src_dmg_mult2 = 1.0 + source.combat_details.combat_stats.damage_taken;
            let thorn_mult = tgt_task_mult * src_dmg_mult2;

            let thorn_roll = Self::random_int(
                1.0,
                thorn_mult * target.combat_details.defensive_max_damage * (1.0 + target_resistance / 100.0) * target_thorn_power,
            ) as f64;
            raw_thorn_damage = thorn_roll.ceil() as i64;
            let mitigated = (src_dmg_taken * thorn_roll).ceil() as i64;
            thorn_damage_done = mitigated.min(source.combat_details.current_hitpoints);
            source.combat_details.current_hitpoints -= thorn_damage_done;
        }

        // Retaliation
        let mut retaliation_damage_done = 0i64;
        let mut raw_retaliation_damage = 0i64;
        if target.combat_details.combat_stats.retaliation > 0.0 {
            let ret_hit_chance = target.combat_details.smash_accuracy_rating.powf(1.4)
                / (target.combat_details.smash_accuracy_rating.powf(1.4) + source.combat_details.smash_evasion_rating.powf(1.4));

            if ret_hit_chance > rng.gen::<f64>() {
                let mut src_eff_armor = source.combat_details.total_armor;
                if src_eff_armor > 0.0 {
                    src_eff_armor /= 1.0 + target.combat_details.combat_stats.armor_penetration;
                }
                let src_dmg_taken = if src_eff_armor < 0.0 {
                    (100.0 - src_eff_armor) / 100.0
                } else {
                    100.0 / (100.0 + src_eff_armor)
                };

                let ret_task_mult = 1.0 + target.combat_details.combat_stats.task_damage;
                let src_dmg_mult2 = 1.0 + source.combat_details.combat_stats.damage_taken;
                let ret_mult = ret_task_mult * src_dmg_mult2;

                let premit = damage_roll.min(target.combat_details.defensive_max_damage * 5.0);
                let ret_min = ret_mult * target.combat_details.combat_stats.retaliation * premit;
                let ret_max = ret_mult * target.combat_details.combat_stats.retaliation * (target.combat_details.defensive_max_damage + premit);
                let ret_roll = Self::random_int(ret_min, ret_max) as f64;
                raw_retaliation_damage = ret_roll.ceil() as i64;
                let mitigated = (src_dmg_taken * ret_roll).ceil() as i64;
                retaliation_damage_done = mitigated.min(source.combat_details.current_hitpoints);
                source.combat_details.current_hitpoints -= retaliation_damage_done;
            }
        }

        // Life steal
        let mut life_steal_heal = 0i64;
        if ability_effect.is_none() && did_hit && source.combat_details.combat_stats.life_steal > 0.0 {
            life_steal_heal = source.add_hitpoints((source.combat_details.combat_stats.life_steal * damage_done as f64).floor() as i64);
        }

        // HP drain (ability)
        let mut hp_drain = 0i64;
        if let Some(ae) = ability_effect {
            if did_hit && ae.hp_drain_ratio > 0.0 {
                let amp = 1.0 + source.combat_details.combat_stats.healing_amplify;
                let drain = (ae.hp_drain_ratio * damage_done as f64 * amp).floor() as i64;
                hp_drain = source.add_hitpoints(drain);
            }
        }

        // Mana leech
        let mut mana_leech_mana = 0i64;
        if ability_effect.is_none() && did_hit && source.combat_details.combat_stats.mana_leech > 0.0 {
            mana_leech_mana = source.add_manapoints((source.combat_details.combat_stats.mana_leech * damage_done as f64).floor() as i64);
        }

        // Debuff damage: portion of damage_done caused by target's damage_taken buff
        // damage_done = base * (1 + dt), so debuff_portion = damage_done * dt / (1 + dt)
        let dt = target.combat_details.combat_stats.damage_taken;
        let debuff_damage = if dt > 0.0 && damage_done > 0 {
            (damage_done as f64 * dt / (1.0 + dt)).round() as i64
        } else { 0 };

        // Resistance debuff attribution: damage from armor/resistance reductions.
        // Map damage_type -> the buff_type that reduces that resistance.
        let resist_buff_type = match damage_type.as_str() {
            "/damage_types/physical" => "/buff_types/armor",
            "/damage_types/water"    => "/buff_types/water_resistance",
            "/damage_types/nature"   => "/buff_types/nature_resistance",
            "/damage_types/fire"     => "/buff_types/fire_resistance",
            _                        => "",
        };
        let mut resist_debuff_damage: Vec<(String, i64)> = Vec::new();
        if !resist_buff_type.is_empty() && damage_done > 0 {
            // Collect all debuff buffs of the matching type on the target (negative flat or ratio)
            let debuff_buffs: Vec<(String, f64, f64)> = target.combat_buffs.iter()
                .filter(|(_, b)| b.type_hrid == resist_buff_type && (b.flat_boost < 0.0 || b.ratio_boost < 0.0))
                .map(|(k, b)| (k.clone(), b.flat_boost, b.ratio_boost))
                .collect();

            if !debuff_buffs.is_empty() {
                // Compute damage ratio with current (debuffed) resistance
                let pen_actual = if source_penetration > 0.0 && target_resistance > 0.0 {
                    target_resistance / (1.0 + source_penetration)
                } else { target_resistance };
                let ratio_actual = if pen_actual < 0.0 { (100.0 - pen_actual) / 100.0 }
                                   else { 100.0 / (100.0 + pen_actual) };

                for (unique, flat, ratio_b) in &debuff_buffs {
                    // Counterfactual resistance without this one debuff buff
                    // (flat_boost and ratio_boost are negative, so subtract them)
                    let res_without = target_resistance - flat - ratio_b * target_resistance;
                    let pen_without = if source_penetration > 0.0 && res_without > 0.0 {
                        res_without / (1.0 + source_penetration)
                    } else { res_without };
                    let ratio_without = if pen_without < 0.0 { (100.0 - pen_without) / 100.0 }
                                        else { 100.0 / (100.0 + pen_without) };
                    // Damage attributable to this buff = damage * (1 - ratio_without/ratio_actual)
                    let contribution = (damage_done as f64 * (1.0 - ratio_without / ratio_actual)).round() as i64;
                    if contribution > 0 {
                        resist_debuff_damage.push((unique.clone(), contribution));
                    }
                }
            }
        }

        AttackResult {
            damage_done,
            did_hit,
            thorn_damage_done,
            thorn_type,
            retaliation_damage_done,
            life_steal_heal,
            hp_drain,
            mana_leech_mana,
            is_crit,
            debuff_damage,
            resist_debuff_damage,
            raw_damage,
            raw_thorn_damage,
            raw_retaliation_damage,
        }
    }

    pub fn process_heal(source: &CombatUnit, ability_effect: &AbilityEffect, target: &mut CombatUnit) -> i64 {
        let amp = 1.0 + source.combat_details.combat_stats.healing_amplify;
        let magic_max = source.combat_details.magic_max_damage;
        let min_heal = amp * (1.0 + ability_effect.damage_flat);
        let max_heal = amp * (ability_effect.damage_ratio * magic_max + ability_effect.damage_flat);
        let heal = Self::random_int(min_heal, max_heal);
        target.add_hitpoints(heal)
    }

    pub fn process_revive(source: &CombatUnit, ability_effect: &AbilityEffect, target: &mut CombatUnit) -> i64 {
        let healed = Self::process_heal(source, ability_effect, target);
        target.combat_details.current_manapoints = target.combat_details.max_manapoints;
        target.clear_ccs();
        healed
    }

    pub fn process_spend_hp(source: &mut CombatUnit, ability_effect: &AbilityEffect) -> i64 {
        let current = source.combat_details.current_hitpoints;
        let spent = (current as f64 * ability_effect.spend_hp_ratio).floor() as i64;
        source.combat_details.current_hitpoints -= spent;
        spent
    }

    pub fn calculate_tick_value(total_value: i64, total_ticks: i32, current_tick: i32) -> i64 {
        let current_sum = (current_tick as i64 * total_value) / total_ticks as i64;
        let previous_sum = ((current_tick as i64 - 1) * total_value) / total_ticks as i64;
        current_sum - previous_sum
    }
}