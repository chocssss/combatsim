mod combatsimulator;
mod optimizer;

use std::io::{self, Read};
use serde_json::{json, Value};
use rayon::prelude::*;
use combatsimulator::{
    combat_simulator::CombatSimulator,
    combat_unit::CombatUnit,
    player::PlayerExt,
    zone::Zone,
    labyrinth::Labyrinth,
    buff::Buff,
    sim_result::SimResult,
};

// -- Input conversion (game export -> simulator DTO) ---------------------------

pub const COMBAT_SLOTS: &[(&str, &str)] = &[
    ("/item_locations/head",      "/equipment_types/head"),
    ("/item_locations/body",      "/equipment_types/body"),
    ("/item_locations/legs",      "/equipment_types/legs"),
    ("/item_locations/feet",      "/equipment_types/feet"),
    ("/item_locations/hands",     "/equipment_types/hands"),
    ("/item_locations/main_hand", "/equipment_types/main_hand"),
    ("/item_locations/two_hand",  "/equipment_types/two_hand"),
    ("/item_locations/off_hand",  "/equipment_types/off_hand"),
    ("/item_locations/pouch",     "/equipment_types/pouch"),
    ("/item_locations/back",      "/equipment_types/back"),
    ("/item_locations/charm",     "/equipment_types/charm"),
    ("/item_locations/neck",      "/equipment_types/neck"),
    ("/item_locations/ring",      "/equipment_types/ring"),
    ("/item_locations/earrings",  "/equipment_types/earrings"),
];

fn convert_equipment(equip_list: &Value) -> Value {
    let mut result = json!({});
    let obj = result.as_object_mut().unwrap();
    for (_, slot) in COMBAT_SLOTS {
        obj.insert(slot.to_string(), Value::Null);
    }
    if let Some(arr) = equip_list.as_array() {
        for item in arr {
            let loc = item["itemLocationHrid"].as_str().unwrap_or("");
            if let Some(&(_, slot)) = COMBAT_SLOTS.iter().find(|(l, _)| *l == loc) {
                obj.insert(slot.to_string(), json!({
                    "hrid": item["itemHrid"],
                    "enhancementLevel": item["enhancementLevel"]
                }));
            }
        }
    }
    result
}

fn convert_consumables(items: &Value, trigger_map: &Value) -> Value {
    let empty = vec![];
    let arr = items.as_array().unwrap_or(&empty);
    let mut result = Vec::new();
    for item in arr.iter().take(3) {
        let hrid = item["itemHrid"].as_str().unwrap_or("");
        let triggers = trigger_map.get(hrid)
            .filter(|t| !t.is_null())
            .cloned()
            .unwrap_or(json!([]));
        result.push(json!({ "hrid": hrid, "triggers": triggers }));
    }
    while result.len() < 3 { result.push(Value::Null); }
    Value::Array(result)
}

fn convert_abilities(abilities: &Value, trigger_map: &Value) -> Value {
    let empty = vec![];
    let arr = abilities.as_array().unwrap_or(&empty);
    let mut result = Vec::new();
    for ab in arr.iter().take(5) {
        let hrid = ab["abilityHrid"].as_str().unwrap_or("");
        let level = ab["level"].as_i64()
            .or_else(|| ab["level"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(1);
        let triggers = trigger_map.get(hrid)
            .filter(|t| !t.is_null())
            .cloned()
            .unwrap_or(json!([]));
        result.push(json!({ "hrid": hrid, "level": level, "triggers": triggers }));
    }
    while result.len() < 4 { result.push(Value::Null); }
    Value::Array(result)
}

fn convert_achievements(achievements: &Value) -> Value {
    let mut result = json!({});
    if let Some(obj) = achievements.as_object() {
        let out = result.as_object_mut().unwrap();
        for (k, v) in obj {
            if v.as_bool().unwrap_or(false) {
                out.insert(k.clone(), json!(true));
            }
        }
    }
    result
}

fn convert_player(export: &Value, name: &str) -> Value {
    let player = &export["player"];
    let trigger_map = &export["triggerMap"];
    let food_items  = &export["food"]["/action_types/combat"];
    let drink_items = &export["drinks"]["/action_types/combat"];

    json!({
        "hrid": name,
        "staminaLevel":      player["staminaLevel"],
        "intelligenceLevel": player["intelligenceLevel"],
        "attackLevel":       player["attackLevel"],
        "meleeLevel":        player["meleeLevel"],
        "defenseLevel":      player["defenseLevel"],
        "rangedLevel":       player["rangedLevel"],
        "magicLevel":        player["magicLevel"],
        "equipment":   convert_equipment(&player["equipment"]),
        "food":        convert_consumables(food_items, trigger_map),
        "drinks":      convert_consumables(drink_items, trigger_map),
        "abilities":   convert_abilities(&export["abilities"], trigger_map),
        "houseRooms":  export["houseRooms"],
        "achievements": convert_achievements(&export["achievements"]),
        "debuffOnLevelGap": 0.0
    })
}

fn parse_export(raw: &Value) -> Vec<Value> {
    if raw.get("player").is_some() {
        let name = raw["name"].as_str().unwrap_or("player").to_string();
        vec![convert_player(raw, &name)]
    } else if raw.as_object().map(|o| o.keys().all(|k| k.parse::<u32>().is_ok())).unwrap_or(false) {
        let obj = raw.as_object().unwrap();
        let mut keys: Vec<u32> = obj.keys().filter_map(|k| k.parse().ok()).collect();
        keys.sort();
        keys.iter().enumerate().map(|(i, k)| {
            let val_str = obj[&k.to_string()].as_str().unwrap_or("{}");
            let val: Value = serde_json::from_str(val_str).unwrap_or(json!({}));
            let name = val["name"].as_str()
                .unwrap_or(&format!("player{}", i + 1))
                .to_string();
            convert_player(&val, &name)
        }).collect()
    } else {
        raw["players"].as_array().cloned().unwrap_or_default()
    }
}

// -- SimResult merging ---------------------------------------------------------

fn merge_results(results: Vec<SimResult>) -> SimResult {
    assert!(!results.is_empty());
    let n = results.len();

    // Use the first result as the base for metadata, then accumulate counts.
    let mut merged = SimResult::new(
        results[0].zone_name.as_deref(),
        results[0].difficulty_tier,
        results[0].labyrinth_name.as_deref(),
        results[0].room_level,
        results[0].number_of_players,
        results[0].is_labyrinth,
    );
    merged.is_dungeon    = results[0].is_dungeon;
    merged.zone_name     = results[0].zone_name.clone();
    merged.difficulty_tier = results[0].difficulty_tier;

    for r in &results {
        // Summed integer fields
        merged.encounters         += r.encounters;
        merged.dungeons_completed += r.dungeons_completed;
        merged.dungeons_failed    += r.dungeons_failed;
        merged.laby_attempt_count += r.laby_attempt_count;
        merged.simulated_time     += r.simulated_time;        merged.last_encounter_finish_time += r.last_encounter_finish_time;
        merged.last_dungeon_finish_time   += r.last_dungeon_finish_time;

        // Max wave reached: take the max across runs
        if r.max_wave_reached > merged.max_wave_reached {
            merged.max_wave_reached = r.max_wave_reached;
        }
        merged.max_enrage_stack = merged.max_enrage_stack.max(r.max_enrage_stack);

        // Dungeon times: accumulate for averaging
        merged.min_dungeon_time += r.min_dungeon_time;
        merged.max_dungeon_time += r.max_dungeon_time;

        // deaths: sum counts
        for (unit, count) in &r.deaths {
            *merged.deaths.entry(unit.clone()).or_insert(0) += count;
        }
        for (unit, count) in &r.player_deaths {
            *merged.player_deaths.entry(unit.clone()).or_insert(0) += count;
        }

        // experience_gained: sum per player per skill
        for (player, skills) in &r.experience_gained {
            let p = merged.experience_gained.entry(player.clone()).or_default();
            for (skill, xp) in skills {
                *p.entry(skill.clone()).or_insert(0.0) += xp;
            }
        }

        // consumables_used: sum counts
        for (player, items) in &r.consumables_used {
            let p = merged.consumables_used.entry(player.clone()).or_default();
            for (item, count) in items {
                *p.entry(item.clone()).or_insert(0) += count;
            }
        }

        // hitpoints_gained / manapoints_gained / hitpoints_spent / mana_used: sum
        for (unit, sources) in &r.hitpoints_gained {
            let u = merged.hitpoints_gained.entry(unit.clone()).or_default();
            for (src, amt) in sources {
                *u.entry(src.clone()).or_insert(0) += amt;
            }
        }
        for (unit, sources) in &r.manapoints_gained {
            let u = merged.manapoints_gained.entry(unit.clone()).or_default();
            for (src, amt) in sources {
                *u.entry(src.clone()).or_insert(0) += amt;
            }
        }
        for (unit, sources) in &r.hitpoints_spent {
            let u = merged.hitpoints_spent.entry(unit.clone()).or_default();
            for (src, amt) in sources {
                *u.entry(src.clone()).or_insert(0) += amt;
            }
        }

        // player_damage_dealt / player_damage_taken: sum across runs
        for (player, amt) in &r.player_damage_dealt {
            *merged.player_damage_dealt.entry(player.clone()).or_insert(0) += amt;
        }
        for (player, amt) in &r.player_damage_taken {
            *merged.player_damage_taken.entry(player.clone()).or_insert(0) += amt;
        }
        for (player, by_source) in &r.player_damage_taken_by_source {
            let m = merged.player_damage_taken_by_source.entry(player.clone()).or_default();
            for (src, amt) in by_source { *m.entry(src.clone()).or_insert(0) += amt; }
        }
        for (player, by_ability) in &r.player_damage_taken_by_ability {
            let m = merged.player_damage_taken_by_ability.entry(player.clone()).or_default();
            for (ab, amt) in by_ability { *m.entry(ab.clone()).or_insert(0) += amt; }
        }
        for (player, by_ability) in &r.player_damage_dealt_by_ability {
            let m = merged.player_damage_dealt_by_ability.entry(player.clone()).or_default();
            for (ab, amt) in by_ability { *m.entry(ab.clone()).or_insert(0) += amt; }
        }
        for (player, abilities) in &r.mana_used {
            let p = merged.mana_used.entry(player.clone()).or_default();
            for (ab, amt) in abilities {
                *p.entry(ab.clone()).or_insert(0) += amt;
            }
        }

        // time_spent_alive: merge by monster hrid
        for entry in &r.time_spent_alive {
            if let Some(existing) = merged.time_spent_alive.iter_mut().find(|e| e.name == entry.name) {
                existing.time_spent_alive += entry.time_spent_alive;
                existing.count            += entry.count;
                existing.alive             = entry.alive; // last run wins
                existing.spawned_at        = entry.spawned_at;
            } else {
                merged.time_spent_alive.push(entry.clone());
            }
        }

        // player_ran_out_of_mana: OR across runs (true if any run ran out)
        for (player, oom) in &r.player_ran_out_of_mana {
            let e = merged.player_ran_out_of_mana.entry(player.clone()).or_insert(false);
            *e = *e || *oom;
        }

        // player_ran_out_of_mana_time: accumulate total_time_for_out_of_mana
        for (player, entry) in &r.player_ran_out_of_mana_time {
            let e = merged.player_ran_out_of_mana_time.entry(player.clone())
                .or_insert_with(|| combatsimulator::sim_result::ManaTimeEntry {
                    is_out_of_mana: false,
                    start_time_for_out_of_mana: 0,
                    total_time_for_out_of_mana: 0,
                });
            e.total_time_for_out_of_mana += entry.total_time_for_out_of_mana;
            e.is_out_of_mana = e.is_out_of_mana || entry.is_out_of_mana;
        }

        // Per-player scalar fields: average across runs (accumulate, divide after)
        for (player, val) in &r.debuff_on_level_gap {
            *merged.debuff_on_level_gap.entry(player.clone()).or_insert(0.0) += val;
        }
        for (player, val) in &r.drop_rate_multiplier {
            *merged.drop_rate_multiplier.entry(player.clone()).or_insert(0.0) += val;
        }
        for (player, val) in &r.rare_find_multiplier {
            *merged.rare_find_multiplier.entry(player.clone()).or_insert(0.0) += val;
        }
        for (player, val) in &r.combat_drop_quantity {
            *merged.combat_drop_quantity.entry(player.clone()).or_insert(0.0) += val;
        }
        for (player, val) in &r.loot_value {
            *merged.loot_value.entry(player.clone()).or_insert(0.0) += val;
        }
        for (chest, val) in &r.chest_count {
            *merged.chest_count.entry(chest.clone()).or_insert(0.0) += val;
        }
        for (player, by_debuff) in &r.debuff_damage_dealt {
            let p = merged.debuff_damage_dealt.entry(player.clone()).or_default();
            for (debuff, val) in by_debuff {
                *p.entry(debuff.clone()).or_insert(0) += val;
            }
        }
    }

    // Average the per-player scalar fields (they're the same every run but average anyway)
    let n_f = n as f64;
    for v in merged.debuff_on_level_gap.values_mut()    { *v /= n_f; }
    for v in merged.drop_rate_multiplier.values_mut()   { *v /= n_f; }
    for v in merged.rare_find_multiplier.values_mut()   { *v /= n_f; }
    for v in merged.combat_drop_quantity.values_mut()   { *v /= n_f; }

    // simulated_time is the TOTAL across all runs so that
    // damage / simulated_time gives the correct per-second rate.
    // Other time fields are averaged (they represent per-dungeon stats).
    merged.last_encounter_finish_time /= n as i64;
    merged.last_dungeon_finish_time   /= n as i64;
    merged.min_dungeon_time           /= n as i64;
    merged.max_dungeon_time           /= n as i64;

    merged
}

// -- Arg parsing ---------------------------------------------------------------

fn print_usage() {
    eprintln!("MWI Combat Simulator");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("  cat export.json | ./mwi_combat_simulator --zone ZONE [options]");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("  --zone ZONE       Zone hrid, e.g. /actions/combat/sorcerers_tower  (required)");
    eprintln!("  --tier N          Difficulty tier 0-4 (default: 0)");
    eprintln!("  --hours N         Simulation hours (default: 24)");
    eprintln!("  --input FILE      Read player JSON from FILE instead of stdin");
    eprintln!("  --market          Fetch live market prices from milkywayidle.com for loot values");
    eprintln!("  --all-zones       Run all multi-monster/boss zones (planets + Sorcerer's Tower etc.) at tiers 0-4, sorted by XP/hr");
    eprintln!("  --optimize PLAYER Optimise equipment/skills/abilities for the named player (add --market for accurate prices)");
    eprintln!("  --runs N          Number of parallel simulation runs to average (default: 1)");
    eprintln!("  --input FILE      Read player JSON from FILE instead of stdin");
    eprintln!("  --moo-pass        Apply Moo Pass XP bonus");
    eprintln!("  --com-exp N       Community XP buff level");
    eprintln!("  --com-drop N      Community drop buff level");
    eprintln!("  --seal NAME       Apply a seal buff (repeatable). Names: seal_of_wisdom,");
    eprintln!("                    seal_of_damage, seal_of_attack_speed, seal_of_cast_speed,");
    eprintln!("                    seal_of_critical_rate, seal_of_combat_drop, seal_of_rare_find");
    eprintln!("  --pretty          Pretty-print JSON output");
    eprintln!("  --simple          Human-readable summary (DPS, XP/hr, encounters, mana, deaths, loot)");
    eprintln!("  --custom-monster FILE  Load a custom monster JSON file (repeatable).");
    eprintln!("  --list-zones      List all available combat zones and exit");
    eprintln!("  --guild           Guild trial staircase for a trial_* --zone: climbs guild");
    eprintln!("                    level 100, 110, 120, ... up to 300, one attempt per tier");
    eprintln!("                    with full HP/MP and cooldowns restored between tiers.");
    eprintln!("                    Stops on the first wipe or after 1 simulated hour total.");
    eprintln!("                    Disables all consumables and grants a flat +3% HP/MP");
    eprintln!("                    regen buff. Combine with --runs N to climb the staircase");
    eprintln!("                    N times in parallel and report, per guild level, the");
    eprintln!("                    average attempt duration and % of runs that cleared it.");
}

fn list_zones() {
    let map = combatsimulator::data::action_detail_map();
    let mut zones: Vec<(&String, &Value)> = map.iter()
        .filter(|(k, _)| k.starts_with("/actions/combat/"))
        .collect();
    zones.sort_by_key(|(k, _)| k.as_str());
    eprintln!("{:<55} {:<35} {}", "HRID", "NAME", "TYPE");
    eprintln!("{}", "-".repeat(95));
    for (hrid, action) in &zones {
        let name = action["name"].as_str().unwrap_or("?");
        let is_dungeon = action["combatZoneInfo"]["isDungeon"].as_bool().unwrap_or(false);
        let kind = if is_dungeon { "dungeon" } else { "zone" };
        eprintln!("{:<55} {:<35} {}", hrid, name, kind);
    }
}

struct Args {
    zone:     String,
    tier:     i32,
    hours:    f64,
    runs:     usize,
    moo_pass: bool,
    com_exp:  f64,
    com_drop: f64,
    pretty:   bool,
    simple:   bool,
    custom_monsters: Vec<String>,  // paths to JSON files
    seals:    Vec<String>,
    input_file: Option<String>,
    market_prices: bool,
    all_zones: bool,
    optimize: Option<String>,
    guild: bool,
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--list-zones") { list_zones(); std::process::exit(0); }
    if args.iter().any(|a| a == "--help" || a == "-h") { print_usage(); std::process::exit(0); }

    let mut zone     = String::new();
    let mut tier     = 0i32;
    let mut hours    = 24.0f64;
    let mut runs     = 1usize;
    let mut moo_pass = false;
    let mut com_exp  = 0.0f64;
    let mut com_drop = 0.0f64;
    let mut pretty   = false;
    let mut simple   = false;
    let mut custom_monsters: Vec<String> = Vec::new();
    let mut seals: Vec<String> = Vec::new();
    let mut input_file: Option<String> = None;
    let mut market_prices = false;
    let mut all_zones    = false;
    let mut optimize: Option<String> = None;
    let mut guild = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--zone"     => { i += 1; zone     = args.get(i).cloned().unwrap_or_default(); },
            "--tier"     => { i += 1; tier     = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); },
            "--hours"    => { i += 1; hours    = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(24.0); },
            "--runs"     => { i += 1; runs     = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(1).max(1); },
            "--com-exp"  => { i += 1; com_exp  = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0); },
            "--com-drop" => { i += 1; com_drop = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0); },
            "--moo-pass" => { moo_pass = true; },
            "--pretty"         => { pretty   = true; },
            "--simple"         => { simple   = true; },
            "--custom-monster" => { i += 1; if let Some(s) = args.get(i) { custom_monsters.push(s.clone()); } },
            "--seal"     => { i += 1; if let Some(s) = args.get(i) { seals.push(s.clone()); } },
            "--input"    => { i += 1; if let Some(s) = args.get(i) { input_file = Some(s.clone()); } },
            "--market"   => { market_prices = true; },
            "--all-zones"  => { all_zones = true; },
            "--optimize"   => { i += 1; if let Some(s) = args.get(i) { optimize = Some(s.clone()); } },
            "--guild"      => { guild = true; },
            other if other.starts_with("--") => return Err(format!("Unknown argument: {}", other)),
            _ => {}
        }
        i += 1;
    }

    if zone.is_empty() && !all_zones && optimize.is_none() {
        return Err("--zone is required (or use --all-zones for planet zone comparison).".to_string());
    }
    if zone.starts_with("/actions/combat/trial_") && !guild {
        return Err("Trial zones require --guild to run the guild-level staircase.".to_string());
    }

    Ok(Args { zone, tier, hours, runs, moo_pass, com_exp, com_drop, pretty, simple, seals, custom_monsters, input_file, market_prices, all_zones, optimize, guild })
}

// -- Main ----------------------------------------------------------------------

fn print_simple_output(r: &SimResult) {
    let sim_sec = r.simulated_time as f64 / 1e9;
    let sim_hrs = sim_sec / 3600.0;
    let sim_days = sim_hrs / 24.0;

    // -- Group DPS ------------------------------------------------------
    let group_dps: f64 = r.player_damage_dealt.values().sum::<i64>() as f64 / sim_sec;
    let mut players: Vec<&str> = r.player_damage_dealt.keys().map(|s| s.as_str()).collect();
    players.sort();

    println!("=== MWI Combat Simulator ===");
    println!("Zone: {}", r.zone_name.as_deref().unwrap_or("unknown"));
    println!("Simulated: {:.0}h  ({:.1} days)", sim_hrs, sim_days);
    println!();

    // -- DPS ------------------------------------------------------------
    println!("-- DPS --------------------------------------");
    println!("  Group total:  {:.1}", group_dps);
    for pid in &players {
        let dps = *r.player_damage_dealt.get(*pid).unwrap_or(&0) as f64 / sim_sec;
        println!("  {}: {:.1}", pid, dps);
    }
    println!();

    // -- Debuff DPS -----------------------------------------------------
    if !r.debuff_damage_dealt.is_empty() {
        let total_dps: f64 = r.player_damage_dealt.values().sum::<i64>() as f64 / sim_sec;

        // Collect totals per debuff unique_hrid across all players
        let mut by_debuff: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for by_debuff_map in r.debuff_damage_dealt.values() {
            for (unique, &dmg) in by_debuff_map {
                *by_debuff.entry(unique.clone()).or_insert(0) += dmg;
            }
        }
        // Sort debuffs by total damage descending
        let mut debuff_list: Vec<_> = by_debuff.iter().collect();
        debuff_list.sort_by(|a, b| b.1.cmp(a.1));

        let total_debuff_dps: f64 = debuff_list.iter().map(|(_, &v)| v as f64).sum::<f64>() / sim_sec;
        println!("-- Debuff DPS breakdown ---------------------");
        println!("  Total debuff DPS:  {:.1}  ({:.1}% of group DPS)", total_debuff_dps, total_debuff_dps / total_dps * 100.0);
        println!();

        for (unique, &total_dmg) in &debuff_list {
            let debuff_name = unique.split('/').last().unwrap_or(unique)
                .replace('_', " ");
            let dps = total_dmg as f64 / sim_sec;
            println!("  {} —  {:.1} DPS  ({:.1}% of group)", debuff_name, dps, dps / total_dps * 100.0);
            // Show per-player breakdown
            let mut player_rows: Vec<_> = players.iter().filter_map(|&pid| {
                let dmg = r.debuff_damage_dealt.get(pid)?.get(unique.as_str())?;
                let pdps = *dmg as f64 / sim_sec;
                if pdps > 0.0 {
                    let own_dps = *r.player_damage_dealt.get(pid).unwrap_or(&0) as f64 / sim_sec;
                    Some((pid, pdps, pdps / own_dps * 100.0))
                } else { None }
            }).collect();
            player_rows.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap());
            for (pid, pdps, pct) in player_rows {
                println!("    {}: {:.1} DPS  ({:.1}% of their DPS)", pid, pdps, pct);
            }
            println!();
        }
    }

    // -- XP / hr --------------------------------------------------------
    println!("-- XP / hour --------------------------------");
    for pid in &players {
        let skills = r.experience_gained.get(*pid);
        let total_xp_hr = skills.map(|s| s.values().sum::<f64>()).unwrap_or(0.0) / sim_hrs;
        let skill_str = skills.map(|s| {
            let mut parts: Vec<String> = s.iter()
                .filter(|(_,&v)| v > 0.0)
                .map(|(k, &v)| format!("{}: {:.0}/hr", k.split('/').last().unwrap_or(k), v / sim_hrs))
                .collect();
            parts.sort();
            parts.join(", ")
        }).unwrap_or_default();
        println!("  {} total: {:.0}/hr  [{}]", pid, total_xp_hr, skill_str);
    }
    println!();

    // -- Encounters / hr ------------------------------------------------
    println!("-- Encounters / hour ------------------------");
    let enc_hr = r.encounters as f64 / sim_hrs;
    println!("  Encounters:  {:.1}/hr  ({} total)", enc_hr, r.encounters);
    if r.is_dungeon {
        let dng_hr = r.dungeons_completed as f64 / sim_hrs;
        println!("  Dungeons:    {:.2}/hr  ({} completed, {} failed)",
            dng_hr, r.dungeons_completed, r.dungeons_failed);
    }
    println!();

    // -- Mana -----------------------------------------------------------
    println!("-- Out of mana ------------------------------");
    for pid in &players {
        let oom = r.player_ran_out_of_mana.get(*pid).copied().unwrap_or(false);
        let oom_pct = r.player_ran_out_of_mana_time.get(*pid)
            .map(|e| e.total_time_for_out_of_mana as f64 / (sim_sec * 1e9) * 100.0)
            .unwrap_or(0.0);
        if oom {
            println!("  {}: YES  ({:.1}% of time OOM)", pid, oom_pct);
        } else {
            println!("  {}: no", pid);
        }
    }
    println!();

    // -- Player deaths --------------------------------------------------
    println!("-- Player deaths ----------------------------");
    if r.player_deaths.is_empty() {
        println!("  No player deaths");
    } else {
        let mut pd: Vec<_> = r.player_deaths.iter().collect();
        pd.sort_by_key(|(k, _)| k.as_str());
        for (pid, &count) in &pd {
            let per_day = count as f64 / sim_days;
            println!("  {}: {:.1}/day  ({} total)", pid, per_day, count);
        }
    }
    println!();

    // -- Loot value / day -----------------------------------------------
    println!("-- Loot value / day -------------------------");
    let total_loot_day: f64 = r.loot_value.values().sum::<f64>() / sim_days;
    println!("  Total (all players): {:.0} coins/day", total_loot_day);
    for pid in &players {
        let loot_day = r.loot_value.get(*pid).copied().unwrap_or(0.0) / sim_days;
        if loot_day > 0.0 {
            println!("  {}: {:.0} coins/day", pid, loot_day);
        }
    }
    if !r.chest_count.is_empty() {
        println!("  Chests/day:");
        let mut chests: Vec<_> = r.chest_count.iter().collect();
        chests.sort_by(|a,b| b.1.partial_cmp(a.1).unwrap());
        for (chest, count) in chests {
            let name = chest.split('/').last().unwrap_or(chest);
            println!("    {}: {:.1}/day", name, count / sim_days);
        }
    }
    println!();
}


/// Fetch live market prices from the MWI marketplace API.
/// Returns a map of itemHrid -> price_per_unit (lowestAsk, falling back to highestBid).
/// Returns None if the fetch or parse fails (caller falls back to static sellPrice).
fn fetch_market_prices() -> Option<std::collections::HashMap<String, f64>> {
    use std::io::Read;
    let url = "https://www.milkywayidle.com/game_data/marketplace.json";
    eprintln!("Fetching market prices from {}...", url);

    // Use ureq (or fall back gracefully if not available at link time).
    // We use a raw TCP + HTTP/1.1 request so we need no extra crate --
    // but ureq is already pulled in transitively via many crates. If not,
    // we do a best-effort native TLS call via the std TcpStream path below.
    // In practice the simplest approach is a subprocess call to curl/wget
    // since we control the build environment, but we prefer a pure-Rust path.
    //
    // We use ureq if available; otherwise fall back silently.
    #[cfg(feature = "ureq")]
    {
        match ureq::get(url).call() {
            Ok(resp) => {
                if let Ok(text) = resp.into_string() {
                    return parse_market_json(&text);
                }
            }
            Err(e) => { eprintln!("Market fetch failed: {}", e); return None; }
        }
    }
    #[cfg(not(feature = "ureq"))]
    {
        // Fallback: use curl as a subprocess
        match std::process::Command::new("curl")
            .args(["-s", "--max-time", "10", url])
            .output()
        {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                return parse_market_json(&text);
            }
            Ok(_) => { eprintln!("curl returned non-zero exit for market fetch"); return None; }
            Err(e) => { eprintln!("Failed to run curl for market fetch: {}", e); return None; }
        }
    }
    None
}

fn parse_market_json(text: &str) -> Option<std::collections::HashMap<String, f64>> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let mut prices = std::collections::HashMap::new();

    // MWI marketplace format:
    // { "marketData": { "/items/foo": { "0": { "a": 190, "b": 185 }, "10": {...} } }, "timestamp": N }
    // Keys stored in the returned map:
    //   "/items/foo"     -> price at enhancement 0  (used by loot valuation)
    //   "/items/foo:10"  -> price at enhancement 10 (used by equipment optimizer)
    // "a" = lowestAsk, "b" = highestBid. -1 means no active listing, skip it.
    let market_data = &v["marketData"];
    if let Some(obj) = market_data.as_object() {
        for (hrid, enhancements) in obj {
            if let Some(enh_obj) = enhancements.as_object() {
                for (enh_str, data) in enh_obj {
                    let ask = data["a"].as_f64().filter(|&x| x > 0.0);
                    let bid = data["b"].as_f64().filter(|&x| x > 0.0);
                    let price = match ask.or(bid) { Some(p) => p, None => continue };

                    // Store at "hrid:enh" for all levels
                    prices.insert(format!("{}:{}", hrid, enh_str), price);

                    // Also store plain hrid for enh 0 (backwards compat with loot calc)
                    if enh_str == "0" {
                        prices.insert(hrid.clone(), price);
                    }
                }
            }
        }
    }

    if prices.is_empty() {
        eprintln!("Warning: market JSON parsed but no prices found; check format");
        None
    } else {
        eprintln!("Loaded {} market price entries.", prices.len());
        Some(prices)
    }
}


fn planet_zone_hrids() -> Vec<(String, String)> {
    // Returns (hrid, name) for every non-dungeon non-labyrinth zone that has
    // multiple spawn types or a boss -- the "world boss" style zones
    // (planets + Sorcerer's Tower, Bear With It, Golem Cave, Twilight Zone,
    // Infernal Abyss), sorted by name.
    let actions = combatsimulator::data::action_detail_map();
    let mut zones: Vec<(String, String)> = actions
        .iter()
        .filter(|(_, action)| {
            if action["type"].as_str() != Some("/action_types/combat") { return false; }
            let czi = &action["combatZoneInfo"];
            if !czi.is_object() { return false; }
            if czi["isDungeon"].as_bool().unwrap_or(false) { return false; }
            if czi["isLabyrinth"].as_bool().unwrap_or(false) { return false; }
            let fi = &czi["fightInfo"];
            if !fi.is_object() { return false; }
            let spawn_count = fi["randomSpawnInfo"]["spawns"]
                .as_array().map(|a| a.len()).unwrap_or(0);
            let boss_count = fi["bossSpawns"]
                .as_array().map(|a| a.len()).unwrap_or(0);
            spawn_count > 1 || boss_count > 0
        })
        .map(|(hrid, action)| {
            let name = action["name"].as_str().unwrap_or(hrid).to_string();
            (hrid.clone(), name)
        })
        .collect();
    zones.sort_by(|a, b| a.1.cmp(&b.1));
    zones
}

fn run_all_zones(
    args: &Args,
    player_dtos: &[serde_json::Value],
    extra_buffs: &[combatsimulator::buff::Buff],
    market_prices: Option<&std::collections::HashMap<String, f64>>,
) {
    let zones = planet_zone_hrids();
    let time_limit = (args.hours * 3600.0 * 1e9) as i64;
    const MAX_TIER: i32 = 5;

    // Collect player hrids in input order for stable column ordering.
    let player_hrids: Vec<String> = player_dtos.iter()
        .map(|dto| dto["hrid"].as_str().unwrap_or("player").to_string())
        .collect();
    let multi_player = player_hrids.len() > 1;

    struct Row {
        name: String,
        tier: i32,
        // per-player XP/hr, in player_hrids order
        player_xp: Vec<f64>,
        // per-player gold/hr, in player_hrids order
        player_gold: Vec<f64>,
    }

    // Build flat list of (hrid, name, tier) pairs and run all in parallel.
    let pairs: Vec<(String, String, i32)> = zones
        .iter()
        .flat_map(|(hrid, name)| {
            (0..=MAX_TIER).map(move |tier| (hrid.clone(), name.clone(), tier))
        })
        .collect();

    let market_prices_cloned: Option<std::collections::HashMap<String, f64>> =
        market_prices.cloned();

    let player_hrids_c = player_hrids.clone();
    let mut rows: Vec<Row> = pairs
        .into_par_iter()
        .map(|(zone_hrid, zone_name, tier)| {
            let results: Vec<_> = (0..args.runs)
                .into_iter()
                .map(|_| {
                    let z = Some(combatsimulator::zone::Zone::new(zone_hrid.clone(), tier));
                    let players: Vec<_> = player_dtos.iter().map(|dto| {
                        let mut p = combatsimulator::combat_unit::CombatUnit::create_from_dto(dto);
                        if let Some(ref z2) = z { p.zone_buffs = z2.buffs.clone(); }
                        p.extra_buffs = extra_buffs.to_vec();
                        p
                    }).collect();
                    let mut sim = combatsimulator::combat_simulator::CombatSimulator::new(
                        players, z, None, false,
                        market_prices_cloned.clone(),
                    );
                    sim.simulate(time_limit).clone()
                })
                .collect();

            let merged = merge_results(results);
            let sim_hours = args.hours * args.runs as f64;

            let player_xp: Vec<f64> = player_hrids_c.iter().map(|pid| {
                merged.experience_gained.get(pid)
                    .map(|skills| skills.values().sum::<f64>())
                    .unwrap_or(0.0) / sim_hours
            }).collect();

            let player_gold: Vec<f64> = player_hrids_c.iter().map(|pid| {
                merged.loot_value.get(pid).copied().unwrap_or(0.0) / sim_hours
            }).collect();

            Row { name: zone_name, tier, player_xp, player_gold }
        })
        .collect();

    // Sort by total XP/hr across all players descending.
    rows.sort_by(|a, b| {
        let ax: f64 = a.player_xp.iter().sum();
        let bx: f64 = b.player_xp.iter().sum();
        bx.partial_cmp(&ax).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Print header
    if multi_player {
        // Header row 1: zone/tier, then player names spanning XP+Gold columns
        print!("{:<30} {:>4}", "Zone", "T");
        for pid in &player_hrids {
            let label = if pid.len() > 24 { &pid[..24] } else { pid.as_str() };
            print!("  {:<24}", label);
        }
        println!();
        // Header row 2: XP/hr Gold/hr per player
        print!("{:<30} {:>4}", "", "");
        for _ in &player_hrids {
            print!("  {:>11} {:>11}", "XP/hr", "Gold/hr");
        }
        println!();
        println!("{}", "-".repeat(36 + player_hrids.len() * 26));
    } else {
        println!("{:<30} {:>4} {:>14} {:>14}", "Zone", "T", "XP/hr", "Gold/hr");
        println!("{}", "-".repeat(66));
    }

    for r in &rows {
        if multi_player {
            print!("{:<30} {:>4}", r.name, r.tier);
            for (xp, gold) in r.player_xp.iter().zip(r.player_gold.iter()) {
                print!("  {:>11.0} {:>11.0}", xp, gold);
            }
            println!();
        } else {
            println!("{:<30} {:>4} {:>14.0} {:>14.0}",
                r.name, r.tier, r.player_xp[0], r.player_gold[0]);
        }
    }
}

const GUILD_START: i32 = 100;
const GUILD_STEP: i32 = 10;
const GUILD_MAX: i32 = 300;
const GUILD_TIME_BUDGET: i64 = 3600 * 1_000_000_000; // 1 simulated hour, ns

struct GuildLevelAttempt {
    guild: i32,
    won: Option<bool>, // Some(true) = cleared, Some(false) = wiped, None = timed out
    duration_ns: i64,
    // % of boss/enemy hitpoints remaining at the moment of a wipe, or when the
    // attempt was cut off by the guild time budget (None only on a clear).
    boss_hp_pct: Option<f64>,
    // Total player deaths during this attempt.
    deaths: i32,
    // Total damage dealt / healing received by the party, summed across all players.
    total_damage_dealt: i64,
    total_healing_done: i64,
}

/// Climbs a trial zone's guild-level staircase (100, 110, 120, ... up to 300) once.
/// Each tier is a single attempt against a fully healed party with cooldowns reset
/// (a fresh CombatSimulator/CombatUnit set does this naturally). The climb is
/// capped at 1 hour of cumulative simulated time and stops the moment the party wipes.
fn run_one_guild_staircase(
    args: &Args,
    player_dtos: &[serde_json::Value],
    extra_buffs: &[combatsimulator::buff::Buff],
    market_prices: Option<&std::collections::HashMap<String, f64>>,
) -> Vec<GuildLevelAttempt> {
    let mut attempts = Vec::new();
    let mut time_used: i64 = 0;
    let mut guild = GUILD_START;

    loop {
        if guild > GUILD_MAX { break; }

        let remaining = GUILD_TIME_BUDGET - time_used;
        if remaining <= 0 { break; }

        let zone = Some(Zone::new(args.zone.clone(), guild));
        let players: Vec<CombatUnit> = player_dtos.iter().map(|dto| {
            let mut player = CombatUnit::create_from_dto(dto);
            if let Some(ref z) = zone { player.zone_buffs = z.buffs.clone(); }
            player.extra_buffs = extra_buffs.to_vec();
            player
        }).collect();

        let mut sim = CombatSimulator::new(players, zone, None, false, market_prices.cloned());
        sim.set_stop_after_dungeon_result(true);
        sim.set_guild_trial_mode(true);
        let result = sim.simulate(remaining).clone();
        time_used += result.simulated_time;

        let won = result.dungeon_attempt_won;
        let deaths: i32 = result.player_deaths.values().sum();
        let total_damage_dealt: i64 = result.player_damage_dealt.values().sum();
        let total_healing_done: i64 = result.hitpoints_gained.values()
            .flat_map(|by_source| by_source.values())
            .sum();
        attempts.push(GuildLevelAttempt {
            guild,
            won,
            duration_ns: result.simulated_time,
            boss_hp_pct: result.enemy_hp_pct_remaining,
            deaths,
            total_damage_dealt,
            total_healing_done,
        });

        if won == Some(true) {
            guild += GUILD_STEP;
        } else {
            break;
        }
    }

    attempts
}

/// Runs `args.runs` independent guild staircase climbs in parallel and reports,
/// per guild level, the average attempt duration and the % of runs that cleared it.
fn run_guild_staircase(
    args: &Args,
    player_dtos: &[serde_json::Value],
    extra_buffs: &[combatsimulator::buff::Buff],
    market_prices: Option<&std::collections::HashMap<String, f64>>,
) {
    let all_runs: Vec<Vec<GuildLevelAttempt>> = (0..args.runs)
        .into_par_iter()
        .map(|_| run_one_guild_staircase(args, player_dtos, extra_buffs, market_prices))
        .collect();

    println!("=== MWI Guild Trial Staircase: {} ({} run{}) ===",
        args.zone, args.runs, if args.runs == 1 { "" } else { "s" });
    println!("{:<8} {:>10} {:>10} {:>14} {:>11} {:>12} {:>12} {:>18}",
        "Guild", "Attempts", "Cleared%", "Avg Duration", "Avg Deaths", "Avg DPS", "Avg HPS", "Avg Boss HP% (wipe/timeout)");
    println!("{}", "-".repeat(113));

    let mut guild = GUILD_START;
    while guild <= GUILD_MAX {
        let level_attempts: Vec<&GuildLevelAttempt> = all_runs.iter()
            .flat_map(|run| run.iter())
            .filter(|a| a.guild == guild)
            .collect();

        if level_attempts.is_empty() { guild += GUILD_STEP; continue; }

        let n = level_attempts.len();
        let cleared = level_attempts.iter().filter(|a| a.won == Some(true)).count();
        let cleared_pct = cleared as f64 / n as f64 * 100.0;
        let avg_duration = level_attempts.iter().map(|a| a.duration_ns as f64).sum::<f64>()
            / n as f64 / 1e9;
        let avg_deaths = level_attempts.iter().map(|a| a.deaths as f64).sum::<f64>() / n as f64;

        let per_attempt_dps_hps = |dur_ns: i64, total: i64| -> f64 {
            if dur_ns > 0 { total as f64 / (dur_ns as f64 / 1e9) } else { 0.0 }
        };
        let avg_dps = level_attempts.iter()
            .map(|a| per_attempt_dps_hps(a.duration_ns, a.total_damage_dealt))
            .sum::<f64>() / n as f64;
        let avg_hps = level_attempts.iter()
            .map(|a| per_attempt_dps_hps(a.duration_ns, a.total_healing_done))
            .sum::<f64>() / n as f64;

        let boss_hp_pcts: Vec<f64> = level_attempts.iter()
            .filter_map(|a| a.boss_hp_pct)
            .collect();
        let avg_boss_hp = if boss_hp_pcts.is_empty() {
            "n/a".to_string()
        } else {
            format!("{:.1}%", boss_hp_pcts.iter().sum::<f64>() / boss_hp_pcts.len() as f64)
        };

        println!("{:<8} {:>10} {:>9.1}% {:>13.1}s {:>11.2} {:>12.0} {:>12.0} {:>27}",
            guild, n, cleared_pct, avg_duration, avg_deaths, avg_dps, avg_hps, avg_boss_hp);
        guild += GUILD_STEP;
    }

    // Per-run summary: highest guild level cleared (0 if the run failed at the start level).
    let highest_cleared: Vec<i32> = all_runs.iter().map(|run| {
        run.iter().filter(|a| a.won == Some(true)).map(|a| a.guild).max().unwrap_or(0)
    }).collect();
    let avg_highest = highest_cleared.iter().sum::<i32>() as f64 / highest_cleared.len() as f64;
    let cap_reached = highest_cleared.iter().filter(|&&g| g == GUILD_MAX).count();

    println!();
    println!("Average highest guild level cleared: {:.1}", avg_highest);
    if args.runs > 1 {
        println!("Runs reaching cap ({}): {}/{}", GUILD_MAX, cap_reached, args.runs);
    }
}

/// Temporary debug utility: dump one player's fully-computed combat_details
/// (after equipment/buffs/abilities applied) as JSON, for cross-checking
/// against another implementation's player stat computation.
fn dump_one_player(name_filter: &str) {
    let raw = { let mut s = String::new(); io::Read::read_to_string(&mut io::stdin(), &mut s).unwrap(); s };
    let input: Value = serde_json::from_str(&raw).unwrap();
    let player_dtos = parse_export(&input);
    for dto in player_dtos.iter() {
        if dto["hrid"].as_str().unwrap_or("") == name_filter {
            let mut player = CombatUnit::create_from_dto(dto);
            player.generate_permanent_buffs();
            player.clear_buffs();
            player.player_update_combat_details();
            println!("{}", serde_json::to_string_pretty(&player.combat_details).unwrap());
            return;
        }
    }
    eprintln!("player '{}' not found", name_filter);
}

/// Temporary debug utility: run ONE fixed-tier guild-trial encounter (no
/// staircase climbing) for a fixed simulated-time budget and dump aggregate
/// combat statistics (damage, hit/miss counts, ability cast counts, stun
/// counts) for cross-checking against another implementation's engine.
fn dump_tier_stats(zone_hrid: &str, tier: i32, seconds: f64) {
    let raw = { let mut s = String::new(); io::Read::read_to_string(&mut io::stdin(), &mut s).unwrap(); s };
    let input: Value = serde_json::from_str(&raw).unwrap();
    let mut player_dtos = parse_export(&input);
    for dto in player_dtos.iter_mut() {
        dto["food"] = json!([Value::Null, Value::Null, Value::Null]);
        dto["drinks"] = json!([Value::Null, Value::Null, Value::Null]);
    }

    let zone = Some(Zone::new(zone_hrid.to_string(), tier));
    let players: Vec<CombatUnit> = player_dtos.iter().map(|dto| {
        let mut player = CombatUnit::create_from_dto(dto);
        if let Some(ref z) = zone { player.zone_buffs = z.buffs.clone(); }
        extra_regen_buffs().iter().for_each(|b| player.extra_buffs.push(b.clone()));
        player
    }).collect();

    let mut sim = CombatSimulator::new(players, zone, None, false, None);
    sim.set_guild_trial_mode(true);
    let result = sim.simulate((seconds * 1e9) as i64).clone();

    let mut player_hits = 0i64;
    let mut player_misses = 0i64;
    let mut monster_hits = 0i64;
    let mut monster_misses = 0i64;
    let mut player_damage_from_attacks = 0i64;
    let mut monster_damage_from_attacks = 0i64;
    let mut ability_casts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut player_ability_casts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let player_names: std::collections::HashSet<String> =
        player_dtos.iter().filter_map(|d| d["hrid"].as_str().map(|s| s.to_string())).collect();

    for (source, targets) in &result.attacks {
        let is_player_source = player_names.contains(source);
        for (_target, abilities) in targets {
            for (ability, dmgmap) in abilities {
                let mut count_for_ability = 0i64;
                for (dmg_str, count) in dmgmap {
                    count_for_ability += *count as i64;
                    if dmg_str == "miss" {
                        if is_player_source { player_misses += *count as i64; } else { monster_misses += *count as i64; }
                    } else {
                        let dmg: i64 = dmg_str.parse().unwrap_or(0);
                        if is_player_source {
                            player_hits += *count as i64;
                            player_damage_from_attacks += dmg * (*count as i64);
                        } else {
                            monster_hits += *count as i64;
                            monster_damage_from_attacks += dmg * (*count as i64);
                        }
                    }
                }
                if !is_player_source {
                    *ability_casts.entry(ability.clone()).or_insert(0) += count_for_ability;
                } else {
                    *player_ability_casts.entry(ability.clone()).or_insert(0) += count_for_ability;
                }
            }
        }
    }

    let total_damage_dealt: i64 = result.player_damage_dealt.values().sum();
    let total_damage_taken: i64 = result.player_damage_taken.values().sum();

    let summary = json!({
        "tier": tier,
        "seconds_requested": seconds,
        "simulated_time_s": result.simulated_time as f64 / 1e9,
        "dungeon_attempt_won": result.dungeon_attempt_won,
        "enemy_hp_pct_remaining": result.enemy_hp_pct_remaining,
        "num_players": player_dtos.len(),
        "total_damage_dealt_by_players": total_damage_dealt,
        "total_damage_taken_by_players": total_damage_taken,
        "player_damage_from_attacks_map": player_damage_from_attacks,
        "monster_damage_from_attacks_map": monster_damage_from_attacks,
        "player_hits": player_hits,
        "player_misses": player_misses,
        "player_accuracy": player_hits as f64 / (player_hits + player_misses).max(1) as f64,
        "monster_hits": monster_hits,
        "monster_misses": monster_misses,
        "monster_accuracy": monster_hits as f64 / (monster_hits + monster_misses).max(1) as f64,
        "monster_ability_casts": ability_casts,
        "player_ability_casts": player_ability_casts,
        "stuns_applied_on_players": result.stuns_applied_on_players,
        "stun_seconds_on_players": result.stun_seconds_on_players,
        "stuns_applied_on_monsters": result.stuns_applied_on_monsters,
        "stun_seconds_on_monsters": result.stun_seconds_on_monsters,
        "total_player_deaths": result.player_deaths.values().sum::<i32>(),
    });
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
}

fn extra_regen_buffs() -> Vec<Buff> {
    vec![
        Buff::inline("/buff_uniques/guild_hp_regen_buff", "/buff_types/hp_regen", 0.0, 0.03, 0),
        Buff::inline("/buff_uniques/guild_mp_regen_buff", "/buff_types/mp_regen", 0.0, 0.03, 0),
    ]
}

fn main() {
    {
        let args: Vec<String> = std::env::args().collect();
        if let Some(pos) = args.iter().position(|a| a == "--dump-player") {
            let name = args.get(pos + 1).cloned().unwrap_or_default();
            dump_one_player(&name);
            std::process::exit(0);
        }
        if let Some(pos) = args.iter().position(|a| a == "--dump-tier-stats") {
            let zone = args.get(pos + 1).cloned().unwrap_or_default();
            let tier: i32 = args.get(pos + 2).and_then(|s| s.parse().ok()).unwrap_or(230);
            let seconds: f64 = args.get(pos + 3).and_then(|s| s.parse().ok()).unwrap_or(120.0);
            dump_tier_stats(&zone, tier, seconds);
            std::process::exit(0);
        }
    }
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            print_usage();
            std::process::exit(1);
        }
    };

    // Load and inject custom monsters BEFORE the data map is first accessed.
    if !args.custom_monsters.is_empty() {
        let mut custom_map: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
        for path in &args.custom_monsters {
            let text = std::fs::read_to_string(path)
                .unwrap_or_else(|e| { eprintln!("Error reading '{}': {}", path, e); std::process::exit(1); });
            let val: serde_json::Value = serde_json::from_str(&text)
                .unwrap_or_else(|e| { eprintln!("Error parsing '{}': {}", path, e); std::process::exit(1); });
            // Accept either a single monster object or an array of monsters
            match &val {
                serde_json::Value::Array(arr) => {
                    for m in arr {
                        let hrid = m["hrid"].as_str().unwrap_or_else(|| {
                            eprintln!("Custom monster missing 'hrid' field in {}", path);
                            std::process::exit(1);
                        }).to_string();
                        eprintln!("Loading custom monster: {}", hrid);
                        custom_map.insert(hrid, m.clone());
                    }
                }
                serde_json::Value::Object(_) => {
                    let hrid = val["hrid"].as_str().unwrap_or_else(|| {
                        eprintln!("Custom monster missing 'hrid' field in {}", path);
                        std::process::exit(1);
                    }).to_string();
                    eprintln!("Loading custom monster: {}", hrid);
                    custom_map.insert(hrid, val);
                }
                _ => {
                    eprintln!("Custom monster file '{}' must be a JSON object or array", path);
                    std::process::exit(1);
                }
            }
        }
        combatsimulator::data::inject_custom_monsters(custom_map);
    }

    let input = match &args.input_file {
        Some(path) => std::fs::read_to_string(path)
            .unwrap_or_else(|e| { eprintln!("Error reading {}: {}", path, e); std::process::exit(1); }),
        None => {
            let mut s = String::new();
            io::stdin().read_to_string(&mut s).expect("Failed to read stdin");
            s
        }
    };
    let raw: Value = serde_json::from_str(&input).expect("Invalid JSON input");

    let mut player_dtos = parse_export(&raw);
    if player_dtos.is_empty() {
        eprintln!("Error: no players found in input");
        std::process::exit(1);
    }

    // --guild: strip all consumables (guild raids forbid food/drinks).
    if args.guild {
        for dto in player_dtos.iter_mut() {
            dto["food"] = json!([Value::Null, Value::Null, Value::Null]);
            dto["drinks"] = json!([Value::Null, Value::Null, Value::Null]);
        }
    }

    // Build extra buffs
    let mut extra_buffs: Vec<Buff> = Vec::new();
    if args.moo_pass {
        extra_buffs.push(Buff::inline("/buff_uniques/experience_moo_pass_buff", "/buff_types/wisdom", 0.0, 0.05, 0));
    }
    if args.com_exp > 0.0 {
        extra_buffs.push(Buff::inline("/buff_uniques/experience_community_buff", "/buff_types/wisdom",
            0.0, 0.005 * (args.com_exp - 1.0) + 0.2, 0));
    }
    if args.com_drop > 0.0 {
        extra_buffs.push(Buff::inline("/buff_uniques/combat_community_buff", "/buff_types/combat_drop_quantity",
            0.0, 0.005 * (args.com_drop - 1.0) + 0.2, 0));
    }
    if args.guild {
        extra_buffs.push(Buff::inline("/buff_uniques/guild_hp_regen_buff", "/buff_types/hp_regen", 0.0, 0.03, 0));
        extra_buffs.push(Buff::inline("/buff_uniques/guild_mp_regen_buff", "/buff_types/mp_regen", 0.0, 0.03, 0));
    }
    // Seals: permanent passive buffs (same values as JS worker.js personalBuffs)
    for seal in &args.seals {
        let normalized = if seal.starts_with("/items/") {
            seal.clone()
        } else {
            format!("/items/{}", seal)
        };
        let buff = match normalized.as_str() {
            "/items/seal_of_attack_speed"  => Some(Buff::inline("/buff_uniques/personal_attack_speed",  "/buff_types/attack_speed",        0.15, 0.0,  0)),
            "/items/seal_of_cast_speed"    => Some(Buff::inline("/buff_uniques/personal_cast_speed",    "/buff_types/cast_speed",          0.0,  0.15, 0)),
            "/items/seal_of_combat_drop"   => Some(Buff::inline("/buff_uniques/personal_combat_drop",   "/buff_types/combat_drop_quantity", 0.0,  0.15, 0)),
            "/items/seal_of_critical_rate" => Some(Buff::inline("/buff_uniques/personal_critical_rate", "/buff_types/critical_rate",        0.0,  0.1,  0)),
            "/items/seal_of_damage"        => Some(Buff::inline("/buff_uniques/personal_damage",        "/buff_types/damage",               0.08, 0.0,  0)),
            "/items/seal_of_rare_find"     => Some(Buff::inline("/buff_uniques/personal_rare_find",     "/buff_types/rare_find",            0.0,  0.6,  0)),
            "/items/seal_of_wisdom"        => Some(Buff::inline("/buff_uniques/personal_wisdom",        "/buff_types/wisdom",               0.0,  0.2,  0)),
            other => { eprintln!("Warning: unknown seal '{}', skipping", other); None }
        };
        if let Some(b) = buff { extra_buffs.push(b); }
    }

    let time_limit = (args.hours * 3600.0 * 1_000_000_000.0) as i64;

    // Warm up the static data caches on the main thread before spawning workers,
    // so threads don't race to initialise OnceLock.
    let _ = combatsimulator::data::action_detail_map();
    let _ = combatsimulator::data::item_detail_map();
    let _ = combatsimulator::data::ability_detail_map();
    let _ = combatsimulator::data::combat_monster_detail_map();
    let _ = combatsimulator::data::house_room_detail_map();
    let _ = combatsimulator::data::achievement_detail_map();
    let _ = combatsimulator::data::achievement_tier_detail_map();
    let _ = combatsimulator::data::combat_style_detail_map();
    let _ = combatsimulator::data::enhancement_level_table();

    // Optionally fetch live market prices once before spawning parallel runs.
    let market_prices: Option<std::collections::HashMap<String, f64>> =
        if args.market_prices { fetch_market_prices() } else { None };

    // --optimize: run upgrade optimisation for a named player, then exit.
    if let Some(ref optimize_player) = args.optimize {
        optimizer::run_optimize(
            &args, &player_dtos, &extra_buffs,
            market_prices.as_ref(),
            optimize_player,
        );
        return;
    }

    // --all-zones: run every non-dungeon zone and print comparison table, then exit.
    if args.all_zones {
        run_all_zones(&args, &player_dtos, &extra_buffs, market_prices.as_ref());
        return;
    }

    // --guild: climb the trial guild-level staircase, then exit.
    if args.guild {
        run_guild_staircase(&args, &player_dtos, &extra_buffs, market_prices.as_ref());
        return;
    }

    // Run `args.runs` independent simulations in parallel.
    let results: Vec<SimResult> = (0..args.runs)
        .into_par_iter()
        .map(|_| {
            let zone = Some(Zone::new(args.zone.clone(), args.tier));
            let players: Vec<CombatUnit> = player_dtos.iter().map(|dto| {
                let mut player = CombatUnit::create_from_dto(dto);
                if let Some(ref z) = zone { player.zone_buffs = z.buffs.clone(); }
                player.extra_buffs = extra_buffs.clone();
                player
            }).collect();

            let mut sim = CombatSimulator::new(players, zone, None, false, market_prices.clone());
            sim.simulate(time_limit).clone()
        })
        .collect();

    if std::env::var("MWI_DEBUG_AUTOATTACK").is_ok() {
        for (i, r) in results.iter().enumerate() {
            if let Some(by_ability) = r.player_damage_dealt_by_ability.get("player1") {
                eprintln!("[RAW run {}] player1 by_ability = {:?}", i, by_ability);
            } else {
                eprintln!("[RAW run {}] player1 NOT PRESENT in player_damage_dealt_by_ability", i);
            }
        }
    }

    let merged = merge_results(results);

    if std::env::var("MWI_DEBUG_AUTOATTACK").is_ok() {
        if let Some(by_ability) = merged.player_damage_dealt_by_ability.get("player1") {
            eprintln!("[MERGED] player1 by_ability = {:?}", by_ability);
        }
    }

    if args.simple {
        print_simple_output(&merged);
        return;
    }

    let output = if args.pretty {
        serde_json::to_string_pretty(&merged)
    } else {
        serde_json::to_string(&merged)
    }.expect("Failed to serialize result");

    println!("{}", output);
}
