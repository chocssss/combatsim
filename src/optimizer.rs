// -- Optimizer ----------------------------------------------------------------
//
// Implements --optimize PLAYER: runs baseline sim then sweeps equipment slots,
// skill levels, and ability levels for the named player, printing two tables:
//
//   Table A+C  Coins  — equipment swaps + ability book purchases, $/DPS
//   Table B    XP     — skill level-ups, XP/DPS + farming hours
//
// All candidate sims run in parallel via rayon.

use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use serde_json::{json, Value};

use crate::combatsimulator::combat_unit::CombatUnit;
use crate::combatsimulator::player::PlayerExt;
use crate::combatsimulator::combat_simulator::CombatSimulator;
use crate::combatsimulator::zone::Zone;
use crate::combatsimulator::buff::Buff;
use crate::SimResult;
use crate::merge_results;
use crate::Args;

// -- helpers -------------------------------------------------------------------

// XP required to reach each level (index 0 = level 1, index 199 = level 200).
// Source: MWI in-game level table.
static XP_TABLE: [i64; 200] = [
    0_i64, 33_i64, 76_i64, 132_i64, 202_i64, 286_i64, 386_i64, 503_i64, 637_i64, 791_i64,
    964_i64, 1159_i64, 1377_i64, 1620_i64, 1891_i64, 2192_i64, 2525_i64, 2893_i64, 3300_i64, 3750_i64,
    4247_i64, 4795_i64, 5400_i64, 6068_i64, 6805_i64, 7618_i64, 8517_i64, 9508_i64, 10604_i64, 11814_i64,
    13151_i64, 14629_i64, 16262_i64, 18068_i64, 20064_i64, 22271_i64, 24712_i64, 27411_i64, 30396_i64, 33697_i64,
    37346_i64, 41381_i64, 45842_i64, 50773_i64, 56222_i64, 62243_i64, 68895_i64, 76242_i64, 84355_i64, 93311_i64,
    103195_i64, 114100_i64, 126127_i64, 139390_i64, 154009_i64, 170118_i64, 187863_i64, 207403_i64, 228914_i64, 252584_i64,
    278623_i64, 307256_i64, 338731_i64, 373318_i64, 411311_i64, 453030_i64, 498824_i64, 549074_i64, 604193_i64, 664632_i64,
    730881_i64, 803472_i64, 882985_i64, 970050_i64, 1065351_i64, 1169633_i64, 1283701_i64, 1408433_i64, 1544780_i64, 1693774_i64,
    1856536_i64, 2034279_i64, 2228321_i64, 2440088_i64, 2671127_i64, 2923113_i64, 3197861_i64, 3497335_i64, 3823663_i64, 4179145_i64,
    4566274_i64, 4987741_i64, 5446463_i64, 5945587_i64, 6488521_i64, 7078945_i64, 7720834_i64, 8418485_i64, 9176537_i64, 10000000_i64,
    11404976_i64, 12904567_i64, 14514400_i64, 16242080_i64, 18095702_i64, 20083886_i64, 22215808_i64, 24501230_i64, 26950540_i64, 29574787_i64,
    32385721_i64, 35395838_i64, 38618420_i64, 42067584_i64, 45758332_i64, 49706603_i64, 53929328_i64, 58444489_i64, 63271179_i64, 68429670_i64,
    73941479_i64, 79829440_i64, 86117783_i64, 92832214_i64, 100000000_i64, 114406130_i64, 130118394_i64, 147319656_i64, 166147618_i64, 186752428_i64,
    209297771_i64, 233962072_i64, 260939787_i64, 290442814_i64, 322702028_i64, 357968938_i64, 396517495_i64, 438646053_i64, 484679494_i64, 534971538_i64,
    589907252_i64, 649905763_i64, 715423218_i64, 786955977_i64, 865044093_i64, 950275074_i64, 1043287971_i64, 1144777804_i64, 1255500373_i64, 1376277458_i64,
    1508002470_i64, 1651646566_i64, 1808265285_i64, 1979005730_i64, 2165114358_i64, 2367945418_i64, 2588970089_i64, 2829786381_i64, 3092129857_i64, 3377885250_i64,
    3689099031_i64, 4027993033_i64, 4396979184_i64, 4798675471_i64, 5235923207_i64, 5711805728_i64, 6229668624_i64, 6793141628_i64, 7406162301_i64, 8073001662_i64,
    8798291902_i64, 9587056372_i64, 10444742007_i64, 11377254401_i64, 12390995728_i64, 13492905745_i64, 14690506120_i64, 15991948361_i64, 17406065609_i64, 18942428633_i64,
    20611406335_i64, 22424231139_i64, 24393069640_i64, 26531098945_i64, 28852589138_i64, 31372992363_i64, 34109039054_i64, 37078841860_i64, 40302007875_i64, 43799759843_i64,
    47595067021_i64, 51712786465_i64, 56179815564_i64, 61025256696_i64, 66280594953_i64, 71979889960_i64, 78159982881_i64, 84860719814_i64, 92125192822_i64, 100000000000_i64,
];

/// Total XP required to reach level n (1-indexed, clamped to 1..=200).
fn xp_at_level(n: i64) -> i64 {
    let idx = (n - 1).clamp(0, 199) as usize;
    XP_TABLE[idx]
}
fn xp_for_levels(base: i64, count: i64) -> i64 {
    xp_at_level(base + count) - xp_at_level(base)
}

/// Run `args.runs` sims for the given player DTOs and zone, return merged result.
fn run_sim(
    player_dtos: &[Value],
    extra_buffs: &[Buff],
    zone_hrid: &str,
    tier: i32,
    time_limit: i64,
    runs: usize,
    market_prices: Option<&std::collections::HashMap<String, f64>>,
) -> SimResult {
    let results: Vec<SimResult> = (0..runs)
        .map(|_| {
            let z = Some(Zone::new(zone_hrid.to_string(), tier));
            let players: Vec<CombatUnit> = player_dtos.iter().map(|dto| {
                let mut p = CombatUnit::create_from_dto(dto);
                if let Some(ref z2) = z { p.zone_buffs = z2.buffs.clone(); }
                p.extra_buffs = extra_buffs.to_vec();
                p
            }).collect();
            let mut sim = CombatSimulator::new(
                players, z, None, false, market_prices.map(|m| m.clone()),
            );
            sim.simulate(time_limit).clone()
        })
        .collect();
    merge_results(results)
}

/// DPS for a single player from a SimResult over args.hours * args.runs simulated hours.
fn player_dps(r: &SimResult, pid: &str, sim_hours: f64) -> f64 {
    r.player_damage_dealt.get(pid).copied().unwrap_or(0) as f64 / (sim_hours * 3600.0)
}

/// Group total DPS from a SimResult.
fn group_dps(r: &SimResult, sim_hours: f64) -> f64 {
    r.player_damage_dealt.values().sum::<i64>() as f64 / (sim_hours * 3600.0)
}

/// XP/hr for a specific skill for a specific player.
fn skill_xp_hr(r: &SimResult, pid: &str, skill: &str, sim_hours: f64) -> f64 {
    r.experience_gained.get(pid)
        .and_then(|m| m.get(skill))
        .copied()
        .unwrap_or(0.0) / sim_hours
}

/// Total XP/hr across all skills for a player.
fn total_xp_hr(r: &SimResult, pid: &str, sim_hours: f64) -> f64 {
    r.experience_gained.get(pid)
        .map(|m| m.values().sum::<f64>())
        .unwrap_or(0.0) / sim_hours
}

/// Detect a player's primary combat style from their weapon's primaryTraining.
/// Returns "melee", "magic", or "ranged".
fn detect_combat_style(dto: &Value) -> &'static str {
    let items = crate::combatsimulator::data::item_detail_map();
    for slot in &["/equipment_types/main_hand", "/equipment_types/two_hand"] {
        if let Some(eq) = dto["equipment"][slot].as_object() {
            if let Some(hrid) = eq.get("hrid").and_then(|h| h.as_str()) {
                let item = match items.get(hrid) { Some(i) => i, None => continue };
                let pt = item["equipmentDetail"]["combatStats"]["primaryTraining"]
                    .as_str().unwrap_or("");
                return match pt {
                    "/skills/magic"   => "magic",
                    "/skills/ranged"  => "ranged",
                    _                 => "melee",
                };
            }
        }
    }
    "melee"
}

/// Offensive stat keys per combat style.
fn style_offensive_keys(style: &str) -> &'static [&'static str] {
    match style {
        "magic"  => &["magicAccuracy", "magicDamage", "castSpeed", "abilityDamage",
                      "waterAmplify", "fireAmplify", "natureAmplify",
                      "waterPenetration", "firePenetration", "naturePenetration"],
        "ranged" => &["rangedAccuracy", "rangedDamage"],
        _        => &["stabAccuracy", "slashAccuracy", "smashAccuracy",
                      "stabDamage", "slashDamage", "smashDamage", "autoAttackDamage",
                      "armorPenetration", "fury"],
    }
}

/// Returns false if an item has offensive stats exclusively for OTHER styles
/// (meaning it provides no offensive benefit to this player's style).
/// Items with zero style-specific offensive stats (pure defense/utility) always return true
/// since they benefit every style equally.
fn is_relevant_for_style(item_hrid: &str, style: &str) -> bool {
    let items = crate::combatsimulator::data::item_detail_map();
    let item = match items.get(item_hrid) { Some(i) => i, None => return true };
    let cs = &item["equipmentDetail"]["combatStats"];

    let other_styles: &[&str] = match style {
        "magic"  => &["ranged", "melee"],
        "ranged" => &["magic", "melee"],
        _        => &["magic", "ranged"],
    };

    // Check if the item has ANY positive offensive stat for the player's style
    let has_own_style = style_offensive_keys(style)
        .iter()
        .any(|k| cs[k].as_f64().unwrap_or(0.0) > 0.001);

    // Check if the item has ANY positive offensive stat for OTHER styles
    let has_other_style = other_styles.iter().any(|other| {
        style_offensive_keys(other).iter()
            .any(|k| cs[k].as_f64().unwrap_or(0.0) > 0.001)
    });

    // Keep if: has own-style offence, OR has no style offence at all (pure utility)
    has_own_style || !has_other_style
}

/// Returns true if the player meets all level requirements for an item.
fn meets_level_req(item_hrid: &str, dto: &Value) -> bool {
    let items = crate::combatsimulator::data::item_detail_map();
    let item = match items.get(item_hrid) { Some(i) => i, None => return false };
    let reqs = match item["equipmentDetail"]["levelRequirements"].as_array() {
        Some(r) => r, None => return true,
    };
    for r in reqs {
        let skill = r["skillHrid"].as_str().unwrap_or("").split('/').last().unwrap_or("");
        let req_level = r["level"].as_f64().unwrap_or(0.0);
        let stat_key = format!("{}Level", skill);
        // camelCase: staminaLevel, intelligenceLevel, attackLevel, meleeLevel, etc.
        let player_level = dto[&stat_key].as_f64().unwrap_or(0.0);
        if player_level < req_level { return false; }
    }
    true
}

/// Highest level requirement across all skills for an item (0 if none).
fn max_level_req(item_hrid: &str) -> f64 {
    let items = crate::combatsimulator::data::item_detail_map();
    let item = match items.get(item_hrid) { Some(i) => i, None => return 0.0 };
    let reqs = match item["equipmentDetail"]["levelRequirements"].as_array() {
        Some(r) => r, None => return 0.0,
    };
    reqs.iter()
        .map(|r| r["level"].as_f64().unwrap_or(0.0))
        .fold(0.0, f64::max)
}

/// Sell value of an item at a given enhancement level: prefers the live market
/// price (same "hrid" / "hrid:enh" keying used for candidate costs), falling
/// back to the item's base NPC sellPrice (enh 0 only) if no market data exists.
fn sell_value(item_hrid: &str, enh: u64, market: Option<&std::collections::HashMap<String, f64>>) -> f64 {
    let key = if enh == 0 { item_hrid.to_string() } else { format!("{}:{}", item_hrid, enh) };
    if let Some(m) = market {
        if let Some(&p) = m.get(&key) {
            if p > 0.0 { return p; }
        }
    }
    if enh == 0 {
        let items = crate::combatsimulator::data::item_detail_map();
        if let Some(item) = items.get(item_hrid) {
            return item["sellPrice"].as_f64().unwrap_or(0.0);
        }
    }
    0.0
}




// -- Equipment candidates ------------------------------------------------------

struct EquipCandidate {
    slot:      String,  // "/equipment_types/head"
    item_hrid: String,
    enh:       u64,
    cost:      f64,
}

fn equipment_candidates(
    player_dto:  &Value,
    style:       &str,
    market: Option<&std::collections::HashMap<String, f64>>,
) -> Vec<EquipCandidate> {
    let items = crate::combatsimulator::data::item_detail_map();
    let mut out = Vec::new();

    // Slots we optimise (skip charm per spec)
    let skip_slots: &[&str] = &["/equipment_types/charm"];

    for (item_hrid, item) in items.iter() {
        let ed = &item["equipmentDetail"];
        if !ed.is_object() { continue; }
        let slot = match ed["type"].as_str() { Some(s) => s, None => continue };
        if skip_slots.contains(&slot) { continue; }
        // Must be a combat slot
        if !crate::COMBAT_SLOTS.iter().any(|(_, s)| *s == slot) { continue; }

        let category = item["categoryHrid"].as_str().unwrap_or("");
        // Skip ability books, seals, etc.
        if category == "/item_categories/ability_book" { continue; }

        // Level req
        if !meets_level_req(item_hrid, player_dto) { continue; }
        // Style relevance: skip items with offensive stats only for other styles
        if !is_relevant_for_style(item_hrid, style) { continue; }

        // Currently equipped item in this slot, if any.
        let equipped = &player_dto["equipment"][slot];
        let equipped_hrid = equipped["hrid"].as_str();
        let equipped_enh = equipped["enhancementLevel"].as_u64().unwrap_or(0);

        // Weapon slots (main_hand / two_hand / off_hand) are mutually exclusive —
        // equipping one clears the others. So when checking the level-gap filter
        // for a candidate in any of these slots, compare against whichever weapon
        // slot is actually occupied, not just the literal matching slot.
        const WEAPON_SLOTS: &[&str] = &[
            "/equipment_types/main_hand",
            "/equipment_types/two_hand",
            "/equipment_types/off_hand",
        ];
        let weapon_equipped_hrid: Option<&str> = if WEAPON_SLOTS.contains(&slot) {
            WEAPON_SLOTS.iter()
                .find_map(|s| player_dto["equipment"][s]["hrid"].as_str())
        } else {
            None
        };
        let level_gap_check_hrid = if WEAPON_SLOTS.contains(&slot) {
            weapon_equipped_hrid
        } else {
            equipped_hrid
        };

        if let Some(eq_hrid) = level_gap_check_hrid {
            // Skip items whose level requirement is 30+ levels below the
            // currently equipped item's requirement — these can never be upgrades.
            let equipped_req = max_level_req(eq_hrid);
            let candidate_req = max_level_req(item_hrid);
            if equipped_req > 0.0 && candidate_req <= equipped_req - 30.0 { continue; }
        }

        // Find all enhancement levels that have a market price.
        // Market map stores "hrid:enh" keys for all enhancement levels,
        // and plain "hrid" as shorthand for enh 0.
        //
        // For refined items at enh 13-15, the Philosopher's Mirror recipe applies:
        //   cost(refined:N) = cost(refined:N-1) + price(base:N-2) + price(mirror)
        // e.g. +13r = refined:12 + base:11 + mirror
        //      +14r = refined:13 + base:12 + mirror
        //      +15r = refined:14 + base:13 + mirror
        // We compute these in order so each level can chain from the previous,
        // using a local price cache that includes both market prices and computed costs.
        let is_refined = item["alchemyDetail"]["unrefineDetail"].is_object();
        let base_item_hrid: Option<String> = if is_refined {
            item["baseItemHrids"].as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        };
        let mirror_price = market
            .and_then(|m| m.get("/items/philosophers_mirror"))
            .copied()
            .unwrap_or(0.0);

        // Local price cache: starts with all market prices for this item,
        // then gets computed mirror costs added as we iterate.
        let mut local_prices: std::collections::HashMap<u64, f64> = std::collections::HashMap::new();
        if let Some(m) = market {
            for enh in 0u64..=20 {
                let key = if enh == 0 { item_hrid.clone() } else { format!("{}:{}", item_hrid, enh) };
                if let Some(&p) = m.get(&key) {
                    if p > 0.0 { local_prices.insert(enh, p); }
                }
            }
        }

        // Pre-compute mirror levels 13-15 in order so each can chain from the previous.
        if is_refined && mirror_price > 0.0 {
            if let Some(ref base_hrid) = base_item_hrid {
                for enh in 13u64..=15 {
                    if local_prices.contains_key(&enh) { continue; } // market price takes priority
                    let prev_cost = match local_prices.get(&(enh - 1)).copied() {
                        Some(p) if p > 0.0 => p,
                        _ => continue, // can't chain without previous level
                    };
                    let base_key = if enh - 2 == 0 { base_hrid.clone() } else { format!("{}:{}", base_hrid, enh - 2) };
                    let base_price = match market.and_then(|m| m.get(&base_key)).copied() {
                        Some(p) if p > 0.0 => p,
                        _ => continue,
                    };
                    local_prices.insert(enh, prev_cost + base_price + mirror_price);
                }
            }
        }

        // Sell value of whatever is currently equipped in this exact slot —
        // subtracted from candidate cost so the displayed/ranked cost reflects
        // the actual net coin outlay (buy new item, sell old one).
        let equipped_sell_value = equipped_hrid
            .map(|h| sell_value(h, equipped_enh, market))
            .unwrap_or(0.0);

        let mut found_any = false;
        for enh in 0u64..=15 {
            // Skip the same item at an equal or lower enhancement level than
            // what's currently equipped in this slot — never an upgrade.
            if let Some(eq_hrid) = equipped_hrid {
                if item_hrid == eq_hrid && enh <= equipped_enh { continue; }
            }
            if let Some(&cost) = local_prices.get(&enh) {
                let net_cost = (cost - equipped_sell_value).max(0.0);
                out.push(EquipCandidate {
                    slot: slot.to_string(),
                    item_hrid: item_hrid.clone(),
                    enh,
                    cost: net_cost,
                });
                found_any = true;
            }
        }
        let _ = found_any;
    }
    out
}

/// Build a player DTO with one equipment slot replaced.
/// Handles two-hand / one-hand mutual exclusion:
///   - equipping a two_hand clears main_hand and off_hand
///   - equipping main_hand or off_hand clears two_hand
fn dto_with_equipment(dto: &Value, slot: &str, item_hrid: &str, enh: u64) -> Value {
    let mut d = dto.clone();
    match slot {
        "/equipment_types/two_hand" => {
            d["equipment"]["/equipment_types/main_hand"] = Value::Null;
            d["equipment"]["/equipment_types/off_hand"]  = Value::Null;
        }
        "/equipment_types/main_hand" | "/equipment_types/off_hand" => {
            d["equipment"]["/equipment_types/two_hand"] = Value::Null;
        }
        _ => {}
    }
    d["equipment"][slot] = json!({ "hrid": item_hrid, "enhancementLevel": enh });
    d
}

// -- Skill candidates ----------------------------------------------------------

const COMBAT_SKILLS: &[(&str, &str)] = &[
    ("attack",        "attackLevel"),
    ("melee",         "meleeLevel"),
    ("defense",       "defenseLevel"),
    ("magic",         "magicLevel"),
    ("ranged",        "rangedLevel"),
    ("stamina",       "staminaLevel"),
    ("intelligence",  "intelligenceLevel"),
];

struct SkillCandidate {
    skill:     &'static str,
    stat_key:  &'static str,
    base:      i64,
    delta:     i64,   // +1..+5
    xp_cost:   i64,
}

/// Returns true if leveling this skill can benefit the given combat style.
/// - "melee", "ranged", and "magic" are the three mutually-exclusive offensive
///   skills — only the one matching the player's current style is relevant.
/// - "attack" boosts accuracy for all styles, so it's always relevant.
/// - "defense", "stamina", and "intelligence" are universal (survivability /
///   mana, relevant to every style), so always included.
fn skill_relevant_for_style(skill: &str, style: &str) -> bool {
    match skill {
        "melee"  => style == "melee",
        "ranged" => style == "ranged",
        "magic"  => style == "magic",
        _        => true, // attack, defense, stamina, intelligence
    }
}

fn skill_candidates(dto: &Value, style: &str) -> Vec<SkillCandidate> {
    let mut out = Vec::new();
    for &(skill, stat_key) in COMBAT_SKILLS {
        if !skill_relevant_for_style(skill, style) { continue; }
        let base = dto[stat_key].as_f64().unwrap_or(1.0) as i64;
        for delta in 1i64..=5 {
            out.push(SkillCandidate {
                skill,
                stat_key,
                base,
                delta,
                xp_cost: xp_for_levels(base, delta),
            });
        }
    }
    out
}

fn dto_with_skill(dto: &Value, stat_key: &str, delta: i64) -> Value {
    let mut d = dto.clone();
    let current = d[stat_key].as_f64().unwrap_or(1.0);
    d[stat_key] = json!(current + delta as f64);
    d
}

// -- House room candidates -----------------------------------------------------

/// Combat-relevant house rooms: (room hrid, short label).
/// Excludes rooms whose buffs only affect gathering/production skills
/// (brewery, dairy_barn, forge, garden, kitchen, laboratory, log_shed,
/// observatory, sewing_parlor, workshop).
const COMBAT_HOUSE_ROOMS: &[(&str, &str)] = &[
    ("/house_rooms/archery_range",  "Archery Range (ranged)"),
    ("/house_rooms/armory",         "Armory (defense)"),
    ("/house_rooms/dining_room",    "Dining Room (stamina)"),
    ("/house_rooms/dojo",           "Dojo (attack)"),
    ("/house_rooms/gym",            "Gym (melee)"),
    ("/house_rooms/library",        "Library (intelligence)"),
    ("/house_rooms/mystical_study", "Mystical Study (magic)"),
];

struct HouseCandidate {
    room_hrid:   String,
    label:       String,
    base_level:  i64,
    target_level: i64,
    cost:        f64,
}

/// Coin-equivalent cost to upgrade a house room from base_level to target_level,
/// summing the upgradeCostsMap entries for each intervening level and pricing
/// each material via the market map (falling back to item sellPrice).
fn house_upgrade_cost(
    room_hrid: &str,
    base_level: i64,
    target_level: i64,
    market: Option<&std::collections::HashMap<String, f64>>,
) -> Option<f64> {
    let rooms = crate::combatsimulator::data::house_room_detail_map();
    let room = rooms.get(room_hrid)?;
    let upgrade_costs = room["upgradeCostsMap"].as_object()?;
    let items = crate::combatsimulator::data::item_detail_map();

    let mut total = 0.0;
    for lvl in (base_level + 1)..=target_level {
        let costs = upgrade_costs.get(&lvl.to_string())?.as_array()?;
        for c in costs {
            let item_hrid = c["itemHrid"].as_str().unwrap_or("");
            let count = c["count"].as_f64().unwrap_or(0.0);
            let price = market.and_then(|m| m.get(item_hrid)).copied()
                .filter(|&p| p > 0.0)
                .or_else(|| items.get(item_hrid).and_then(|i| i["sellPrice"].as_f64()))
                .unwrap_or(0.0);
            total += price * count;
        }
    }
    Some(total)
}

/// Skill leveled by a given combat-relevant house room (used to filter by style,
/// mirroring skill_relevant_for_style: only the room matching the player's
/// offensive style, plus universal rooms, are included).
fn house_room_skill(room_hrid: &str) -> &'static str {
    match room_hrid {
        "/house_rooms/archery_range"  => "ranged",
        "/house_rooms/armory"         => "defense",
        "/house_rooms/dining_room"    => "stamina",
        "/house_rooms/dojo"           => "attack",
        "/house_rooms/gym"            => "melee",
        "/house_rooms/library"        => "intelligence",
        "/house_rooms/mystical_study" => "magic",
        _ => "",
    }
}

fn house_candidates(
    dto: &Value,
    style: &str,
    market: Option<&std::collections::HashMap<String, f64>>,
) -> Vec<HouseCandidate> {
    let mut out = Vec::new();
    let rooms = crate::combatsimulator::data::house_room_detail_map();

    for &(room_hrid, label) in COMBAT_HOUSE_ROOMS {
        let skill = house_room_skill(room_hrid);
        if !skill_relevant_for_style(skill, style) { continue; }

        let room = match rooms.get(room_hrid) { Some(r) => r, None => continue };
        let max_level = room["upgradeCostsMap"].as_object()
            .map(|m| m.keys().filter_map(|k| k.parse::<i64>().ok()).max().unwrap_or(0))
            .unwrap_or(0);

        let base_level = dto["houseRooms"][room_hrid].as_i64().unwrap_or(0);
        if base_level >= max_level { continue; }

        for target_level in (base_level + 1)..=max_level {
            if let Some(cost) = house_upgrade_cost(room_hrid, base_level, target_level, market) {
                out.push(HouseCandidate {
                    room_hrid: room_hrid.to_string(),
                    label: label.to_string(),
                    base_level,
                    target_level,
                    cost,
                });
            }
        }
    }
    out
}

fn dto_with_house_level(dto: &Value, room_hrid: &str, level: i64) -> Value {
    let mut d = dto.clone();
    d["houseRooms"][room_hrid] = json!(level);
    d
}

// -- Ability candidates --------------------------------------------------------

struct AbilityCandidate {
    slot:        usize,  // 0-4
    ability_hrid: String,
    book_hrid:   String,
    base_level:  i64,
    delta:       i64,   // +5, +10, +15, +20
    xp_per_book: i64,
    book_cost:   f64,   // market price per book
    total_cost:  f64,   // books_needed * book_cost
}

fn ability_candidates(
    dto: &Value,
    market_flat: Option<&std::collections::HashMap<String, f64>>,
) -> Vec<AbilityCandidate> {
    let items = crate::combatsimulator::data::item_detail_map();
    let abilities_data = crate::combatsimulator::data::ability_detail_map();
    let mut out = Vec::new();

    let abilities = match dto["abilities"].as_array() { Some(a) => a, None => return out };

    for (slot, ab) in abilities.iter().enumerate().take(5) {
        let ability_hrid = match ab["hrid"].as_str() { Some(h) => h, None => continue };
        let base_level = ab["level"].as_i64()
            .or_else(|| ab["level"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(1);

        // Corresponding book item hrid: /abilities/fierce_aura -> /items/fierce_aura
        let book_hrid = ability_hrid.replace("/abilities/", "/items/");

        // Get XP per book use
        let book_item = match items.get(&book_hrid) { Some(i) => i, None => continue };
        let xp_per_book = book_item["abilityBookDetail"]["experienceGain"]
            .as_i64().unwrap_or(0);
        if xp_per_book == 0 { continue; }

        // Get book market price
        let book_cost = if let Some(mp) = market_flat {
            mp.get(book_hrid.as_str()).copied()
                .filter(|&p| p > 0.0)
                .or_else(|| book_item["sellPrice"].as_f64().filter(|&p| p > 0.0))
        } else {
            book_item["sellPrice"].as_f64().filter(|&p| p > 0.0)
        };
        let book_cost = match book_cost { Some(c) => c, None => continue };

        // Check ability actually scales with level
        let ab_data = match abilities_data.get(ability_hrid) { Some(a) => a, None => continue };
        let scales = ab_data["abilityEffects"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .any(|e| {
                e["baseDamageFlatLevelBonus"].as_f64().unwrap_or(0.0).abs() > 1e-9
                || e["baseDamageRatioLevelBonus"].as_f64().unwrap_or(0.0).abs() > 1e-9
                || e["armorDamageRatioLevelBonus"].as_f64().unwrap_or(0.0).abs() > 1e-9
                || e["buffs"].as_array().unwrap_or(&vec![]).iter().any(|b| {
                    b["flatBoostLevelBonus"].as_f64().unwrap_or(0.0).abs() > 1e-9
                    || b["ratioBoostLevelBonus"].as_f64().unwrap_or(0.0).abs() > 1e-9
                })
            });
        if !scales { continue; }

        for delta in [5i64, 10, 15, 20] {
            let new_level = base_level + delta;
            if new_level > 100 { continue; }
            let xp_cost = xp_for_levels(base_level, delta);
            // books needed (ceiling division)
            let books = (xp_cost + xp_per_book - 1) / xp_per_book;
            let total_cost = books as f64 * book_cost;
            out.push(AbilityCandidate {
                slot,
                ability_hrid: ability_hrid.to_string(),
                book_hrid: book_hrid.clone(),
                base_level,
                delta,
                xp_per_book,
                book_cost,
                total_cost,
            });
        }
    }
    out
}

fn dto_with_ability_level(dto: &Value, slot: usize, delta: i64) -> Value {
    let mut d = dto.clone();
    if let Some(arr) = d["abilities"].as_array_mut() {
        if let Some(ab) = arr.get_mut(slot) {
            let cur = ab["level"].as_i64()
                .or_else(|| ab["level"].as_str().and_then(|s| s.parse().ok()))
                .unwrap_or(1);
            ab["level"] = json!(cur + delta);
        }
    }
    d
}

// -- Result row types ----------------------------------------------------------

struct CoinRow {
    category:    String,
    label:       String,
    cost:        f64,
    p_dps_delta: f64,
    g_dps_delta: f64,
}

struct XpRow {
    skill_or_ability: String,
    current_level:    i64,
    delta:            i64,
    xp_cost:          i64,
    p_dps_delta:      f64,
    g_dps_delta:      f64,
    farm_hrs:         f64,  // xp_cost / relevant_xp_hr_at_baseline
}


/// Print a progress bar to stderr. Call after each completed sim.
fn print_progress(done: usize, total: usize, label: &str) {
    let width = 30usize;
    let filled = if total > 0 { done * width / total } else { width };
    let bar: String = (0..width).map(|i| if i < filled { '█' } else { '░' }).collect();
    eprint!("\r  [{bar}] {done}/{total}  {label}    ");
    if done == total { eprintln!(); }
}

// -- Main entry point ----------------------------------------------------------

pub fn run_optimize(
    args:        &Args,
    player_dtos: &[Value],
    extra_buffs: &[Buff],
    market_prices: Option<&std::collections::HashMap<String, f64>>,
    optimize_player: &str,
) {
    let multi = player_dtos.len() > 1;
    let sim_hours = args.hours * args.runs as f64;
    let time_limit = (args.hours * 3600.0 * 1_000_000_000.0) as i64;
    let zone_hrid = &args.zone;
    let tier = args.tier;

    // Find target player DTO index
    let target_idx = match player_dtos.iter().position(|d| {
        d["hrid"].as_str().unwrap_or("") == optimize_player
    }) {
        Some(i) => i,
        None => {
            eprintln!("Error: player '{}' not found in input", optimize_player);
            std::process::exit(1);
        }
    };
    let target_dto = &player_dtos[target_idx];
    let style = detect_combat_style(target_dto);

    eprintln!("Optimizing '{}' (style={}) on {} T{}",
        optimize_player, style, zone_hrid, tier);

    // -- Baseline --------------------------------------------------------------
    eprintln!("  Running baseline...");
    let baseline = run_sim(
        player_dtos, extra_buffs, zone_hrid, tier,
        time_limit, args.runs, market_prices,
    );
    let base_p_dps = player_dps(&baseline, optimize_player, sim_hours);
    let base_g_dps = group_dps(&baseline, sim_hours);

    // XP/hr per skill for farm_hrs calculation
    let base_skill_xp_hr: std::collections::HashMap<&str, f64> = COMBAT_SKILLS.iter()
        .map(|&(skill, _)| (skill, skill_xp_hr(&baseline, optimize_player, skill, sim_hours)))
        .collect();
    // Total ability XP/hr for farm_hrs (abilities don't have per-ability XP tracking,
    // they share the pool; use total_xp_hr as proxy)
    let base_total_xp_hr = total_xp_hr(&baseline, optimize_player, sim_hours);

    // -- Build candidate lists -------------------------------------------------
    let equip_cands = equipment_candidates(
        target_dto, style, market_prices,
    );
    let skill_cands = skill_candidates(target_dto, style);
    let ability_cands = ability_candidates(target_dto, market_prices);
    let house_cands = house_candidates(target_dto, style, market_prices);

    eprintln!("  Candidates: {} equipment, {} skill, {} ability, {} house",
        equip_cands.len(), skill_cands.len(), ability_cands.len(), house_cands.len());

    // -- Equipment sims --------------------------------------------------------
    let equip_total = equip_cands.len();
    let equip_done  = AtomicUsize::new(0);
    eprintln!("  Equipment ({} candidates):", equip_total);
    let equip_rows: Vec<CoinRow> = equip_cands
        .into_par_iter()
        .map(|c| {
            let mut dtos = player_dtos.to_vec();
            dtos[target_idx] = dto_with_equipment(
                &player_dtos[target_idx], &c.slot, &c.item_hrid, c.enh,
            );
            let r = run_sim(&dtos, extra_buffs, zone_hrid, tier,
                time_limit, args.runs, market_prices);
            let p = player_dps(&r, optimize_player, sim_hours);
            let g = group_dps(&r, sim_hours);

            let slot_short = c.slot.split('/').last().unwrap_or(&c.slot).to_string();
            let item_name = {
                let items = crate::combatsimulator::data::item_detail_map();
                items.get(&c.item_hrid)
                    .and_then(|i| i["name"].as_str())
                    .unwrap_or(&c.item_hrid)
                    .to_string()
            };
            let label = format!("[{}] {} +{}", slot_short, item_name, c.enh);
            let done = equip_done.fetch_add(1, Ordering::Relaxed) + 1;
            print_progress(done, equip_total, &label);
            CoinRow {
                category: "equipment".to_string(),
                label,
                cost: c.cost,
                p_dps_delta: p - base_p_dps,
                g_dps_delta: g - base_g_dps,
            }
        })
        .collect();

    // -- Skill sims ------------------------------------------------------------
    let skill_total = skill_cands.len();
    let skill_done  = AtomicUsize::new(0);
    eprintln!("  Skills ({} candidates):", skill_total);
    let skill_rows: Vec<XpRow> = skill_cands
        .into_par_iter()
        .map(|c| {
            let mut dtos = player_dtos.to_vec();
            dtos[target_idx] = dto_with_skill(&player_dtos[target_idx], c.stat_key, c.delta);
            let r = run_sim(&dtos, extra_buffs, zone_hrid, tier,
                time_limit, args.runs, market_prices);
            let p = player_dps(&r, optimize_player, sim_hours);
            let g = group_dps(&r, sim_hours);

            let xp_hr = base_skill_xp_hr.get(c.skill).copied().unwrap_or(1.0).max(1.0);
            let farm_hrs = c.xp_cost as f64 / xp_hr;

            let label = format!("{} +{}", c.skill, c.delta);
            let done = skill_done.fetch_add(1, Ordering::Relaxed) + 1;
            print_progress(done, skill_total, &label);
            XpRow {
                skill_or_ability: c.skill.to_string(),
                current_level: c.base,
                delta: c.delta,
                xp_cost: c.xp_cost,
                p_dps_delta: p - base_p_dps,
                g_dps_delta: g - base_g_dps,
                farm_hrs,
            }
        })
        .collect();

    // -- Ability sims ----------------------------------------------------------
    let ability_total = ability_cands.len();
    let ability_done  = AtomicUsize::new(0);
    eprintln!("  Abilities ({} candidates):", ability_total);
    let ability_coin_rows: Vec<CoinRow> = ability_cands
        .into_par_iter()
        .map(|c| {
            let mut dtos = player_dtos.to_vec();
            dtos[target_idx] = dto_with_ability_level(
                &player_dtos[target_idx], c.slot, c.delta,
            );
            let r = run_sim(&dtos, extra_buffs, zone_hrid, tier,
                time_limit, args.runs, market_prices);
            let p = player_dps(&r, optimize_player, sim_hours);
            let g = group_dps(&r, sim_hours);

            let items = crate::combatsimulator::data::item_detail_map();
            let book_name = items.get(&c.book_hrid)
                .and_then(|i| i["name"].as_str())
                .unwrap_or(&c.book_hrid)
                .to_string();
            let label = format!("[ability] {} +{}  ({}→{})",
                book_name, c.delta, c.base_level, c.base_level + c.delta);

            let done = ability_done.fetch_add(1, Ordering::Relaxed) + 1;
            print_progress(done, ability_total, &label);
            CoinRow {
                category: "ability".to_string(),
                label,
                cost: c.total_cost,
                p_dps_delta: p - base_p_dps,
                g_dps_delta: g - base_g_dps,
            }
        })
        .collect();

    // -- House room sims ------------------------------------------------------
    let house_total = house_cands.len();
    let house_done  = AtomicUsize::new(0);
    eprintln!("  House rooms ({} candidates):", house_total);
    let house_rows: Vec<CoinRow> = house_cands
        .into_par_iter()
        .map(|c| {
            let mut dtos = player_dtos.to_vec();
            dtos[target_idx] = dto_with_house_level(
                &player_dtos[target_idx], &c.room_hrid, c.target_level,
            );
            let r = run_sim(&dtos, extra_buffs, zone_hrid, tier,
                time_limit, args.runs, market_prices);
            let p = player_dps(&r, optimize_player, sim_hours);
            let g = group_dps(&r, sim_hours);

            let label = format!("[house] {} {}→{}", c.label, c.base_level, c.target_level);
            let done = house_done.fetch_add(1, Ordering::Relaxed) + 1;
            print_progress(done, house_total, &label);
            CoinRow {
                category: "house".to_string(),
                label,
                cost: c.cost,
                p_dps_delta: p - base_p_dps,
                g_dps_delta: g - base_g_dps,
            }
        })
        .collect();

    // -- Print Table A+C (Coins) -----------------------------------------------
    let mut coin_rows: Vec<CoinRow> = equip_rows;
    coin_rows.extend(ability_coin_rows);
    coin_rows.extend(house_rows);
    // Sort by personal DPS gain per million coins, descending
    coin_rows.sort_by(|a, b| {
        let ea = if a.cost > 0.0 { a.p_dps_delta / (a.cost / 1_000_000.0) } else { f64::NEG_INFINITY };
        let eb = if b.cost > 0.0 { b.p_dps_delta / (b.cost / 1_000_000.0) } else { f64::NEG_INFINITY };
        eb.partial_cmp(&ea).unwrap_or(std::cmp::Ordering::Equal)
    });

    println!();
    println!("══ COINS TABLE — {} (style: {}) ═══",
        optimize_player, style);
    println!();
    if multi {
        println!("{:<55} {:>14} {:>10} {:>10} {:>12}",
            "Upgrade", "Cost (coins)", "+P.DPS", "+G.DPS", "P.DPS/1M");
        println!("{}", "-".repeat(105));
    } else {
        println!("{:<55} {:>14} {:>10} {:>12}",
            "Upgrade", "Cost (coins)", "+DPS", "DPS/1M");
        println!("{}", "-".repeat(95));
    }

    for r in &coin_rows {
        if r.p_dps_delta <= 0.0 && r.cost > 0.0 { continue; } // skip downgrades
        let eff = if r.cost > 0.0 { r.p_dps_delta / (r.cost / 1_000_000.0) } else { 0.0 };
        let cost_str = format_coins(r.cost);
        if multi {
            println!("{:<55} {:>14} {:>+10.1} {:>+10.1} {:>12.3}",
                truncate(&r.label, 55), cost_str,
                r.p_dps_delta, r.g_dps_delta, eff);
        } else {
            println!("{:<55} {:>14} {:>+10.1} {:>12.3}",
                truncate(&r.label, 55), cost_str, r.p_dps_delta, eff);
        }
    }

    // -- Print Table B (XP) ---------------------------------------------------
    let mut xp_rows = skill_rows;
    xp_rows.sort_by(|a, b| {
        let ea = if a.xp_cost > 0 { a.p_dps_delta / (a.xp_cost as f64 / 1000.0) } else { f64::NEG_INFINITY };
        let eb = if b.xp_cost > 0 { b.p_dps_delta / (b.xp_cost as f64 / 1000.0) } else { f64::NEG_INFINITY };
        eb.partial_cmp(&ea).unwrap_or(std::cmp::Ordering::Equal)
    });

    println!();
    println!("══ SKILL LEVELS TABLE — {} ═══", optimize_player);
    println!();
    if multi {
        println!("{:<20} {:>8} {:>8} {:>12} {:>10} {:>10} {:>12} {:>10}",
            "Skill", "Current", "+Levels", "XP Cost", "+P.DPS", "+G.DPS", "DPS/1k XP", "Farm hrs");
        println!("{}", "-".repeat(96));
    } else {
        println!("{:<20} {:>8} {:>8} {:>12} {:>10} {:>12} {:>10}",
            "Skill", "Current", "+Levels", "XP Cost", "+DPS", "DPS/1k XP", "Farm hrs");
        println!("{}", "-".repeat(86));
    }
    for r in &xp_rows {
        let eff = if r.xp_cost > 0 {
            r.p_dps_delta / (r.xp_cost as f64 / 1000.0)
        } else { 0.0 };
        let xp_str = format!("{:>12}", format_number(r.xp_cost as f64));
        if multi {
            println!("{:<20} {:>8} {:>+8} {:>12} {:>+10.1} {:>+10.1} {:>12.4} {:>9.1}h",
                r.skill_or_ability, r.current_level, r.delta,
                xp_str, r.p_dps_delta, r.g_dps_delta, eff, r.farm_hrs);
        } else {
            println!("{:<20} {:>8} {:>+8} {:>12} {:>+10.1} {:>12.4} {:>9.1}h",
                r.skill_or_ability, r.current_level, r.delta,
                xp_str, r.p_dps_delta, eff, r.farm_hrs);
        }
    }

    // Print baseline for reference
    println!();
    if multi {
        println!("  Baseline: {:.1} P.DPS  |  {:.1} G.DPS  |  zone={} T{}",
            base_p_dps, base_g_dps, zone_hrid, tier);
    } else {
        println!("  Baseline: {:.1} DPS  |  zone={} T{}",
            base_p_dps, zone_hrid, tier);
    }
    if market_prices.is_none() {
        println!("  Note: --market not used. Equipment costs are static sellPrice (may be very inaccurate).");
        println!("        Ability book costs are static sellPrice (may differ from market by 100x+).");
    }
}

// -- Formatting helpers --------------------------------------------------------

fn format_coins(v: f64) -> String {
    if v >= 1_000_000_000.0 {
        format!("{:.1}B", v / 1_000_000_000.0)
    } else if v >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v >= 1_000.0 {
        format!("{:.0}K", v / 1_000.0)
    } else {
        format!("{:.0}", v)
    }
}

fn format_number(v: f64) -> String {
    let v = v as i64;
    let s = v.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { result.push(','); }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else { format!("{}…", &s[..max-1]) }
}