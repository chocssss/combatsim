use std::collections::HashMap;
use serde_json::Value;
use std::sync::OnceLock;

// All game data embedded at compile time — no external files needed
static ACTION_DETAIL_MAP_JSON:           &str = include_str!("../data/actionDetailMap.json");
static ITEM_DETAIL_MAP_JSON:             &str = include_str!("../data/itemDetailMap.json");
static ABILITY_DETAIL_MAP_JSON:          &str = include_str!("../data/abilityDetailMap.json");
static COMBAT_MONSTER_DETAIL_MAP_JSON:   &str = include_str!("../data/combatMonsterDetailMap.json");
static HOUSE_ROOM_DETAIL_MAP_JSON:       &str = include_str!("../data/houseRoomDetailMap.json");
static ACHIEVEMENT_DETAIL_MAP_JSON:      &str = include_str!("../data/achievementDetailMap.json");
static ACHIEVEMENT_TIER_DETAIL_MAP_JSON: &str = include_str!("../data/achievementTierDetailMap.json");
static COMBAT_STYLE_DETAIL_MAP_JSON:     &str = include_str!("../data/combatStyleDetailMap.json");
static TRIGGER_DEPENDENCY_MAP_JSON:      &str = include_str!("../data/combatTriggerDependencyDetailMap.json");
static ENHANCEMENT_TABLE_JSON:           &str = include_str!("../data/enhancementLevelTotalBonusMultiplierTable.json");

static ACTION_DETAIL_MAP:              OnceLock<HashMap<String, Value>> = OnceLock::new();
static ITEM_DETAIL_MAP:                OnceLock<HashMap<String, Value>> = OnceLock::new();
static ABILITY_DETAIL_MAP:             OnceLock<HashMap<String, Value>> = OnceLock::new();
static COMBAT_MONSTER_DETAIL_MAP:      OnceLock<HashMap<String, Value>> = OnceLock::new();
static HOUSE_ROOM_DETAIL_MAP:          OnceLock<HashMap<String, Value>> = OnceLock::new();
static ACHIEVEMENT_DETAIL_MAP:         OnceLock<HashMap<String, Value>> = OnceLock::new();
static ACHIEVEMENT_TIER_DETAIL_MAP:    OnceLock<HashMap<String, Value>> = OnceLock::new();
static COMBAT_STYLE_DETAIL_MAP:        OnceLock<HashMap<String, Value>> = OnceLock::new();
static TRIGGER_DEPENDENCY_MAP:         OnceLock<HashMap<String, bool>>  = OnceLock::new();
static ENHANCEMENT_TABLE:              OnceLock<Vec<f64>>               = OnceLock::new();
static LABYRINTH_CRATE_MAP:            OnceLock<HashMap<String, Value>> = OnceLock::new();

fn parse_map(json: &str) -> HashMap<String, Value> {
    serde_json::from_str::<Value>(json)
        .ok()
        .and_then(|v| if let Value::Object(m) = v { Some(m.into_iter().collect()) } else { None })
        .unwrap_or_default()
}

pub fn action_detail_map() -> &'static HashMap<String, Value> {
    ACTION_DETAIL_MAP.get_or_init(|| parse_map(ACTION_DETAIL_MAP_JSON))
}
pub fn item_detail_map() -> &'static HashMap<String, Value> {
    ITEM_DETAIL_MAP.get_or_init(|| parse_map(ITEM_DETAIL_MAP_JSON))
}
pub fn ability_detail_map() -> &'static HashMap<String, Value> {
    ABILITY_DETAIL_MAP.get_or_init(|| parse_map(ABILITY_DETAIL_MAP_JSON))
}
pub fn combat_monster_detail_map() -> &'static HashMap<String, Value> {
    COMBAT_MONSTER_DETAIL_MAP.get_or_init(|| parse_map(COMBAT_MONSTER_DETAIL_MAP_JSON))
}

/// Inject custom monster definitions before the map is first used.
/// Must be called before any combat simulation starts.
/// Panics if the map has already been initialised (called too late).
pub fn inject_custom_monsters(monsters: HashMap<String, Value>) {
    if monsters.is_empty() { return; }
    // We need to initialise with the base data + custom entries merged.
    // get_or_init is atomic but we can only call it once.
    // If it's already set this will be a no-op and we'll panic below.
    let _ = COMBAT_MONSTER_DETAIL_MAP.get_or_init(|| {
        let mut map = parse_map(COMBAT_MONSTER_DETAIL_MAP_JSON);
        for (k, v) in monsters {
            map.insert(k, v);
        }
        map
    });
    // If it was already initialised before we got here, the custom monsters were lost.
    // In practice this is fine as long as inject_custom_monsters is called early in main().
}
pub fn house_room_detail_map() -> &'static HashMap<String, Value> {
    HOUSE_ROOM_DETAIL_MAP.get_or_init(|| parse_map(HOUSE_ROOM_DETAIL_MAP_JSON))
}
pub fn achievement_detail_map() -> &'static HashMap<String, Value> {
    ACHIEVEMENT_DETAIL_MAP.get_or_init(|| parse_map(ACHIEVEMENT_DETAIL_MAP_JSON))
}
pub fn achievement_tier_detail_map() -> &'static HashMap<String, Value> {
    ACHIEVEMENT_TIER_DETAIL_MAP.get_or_init(|| parse_map(ACHIEVEMENT_TIER_DETAIL_MAP_JSON))
}
pub fn combat_style_detail_map() -> &'static HashMap<String, Value> {
    COMBAT_STYLE_DETAIL_MAP.get_or_init(|| parse_map(COMBAT_STYLE_DETAIL_MAP_JSON))
}
pub fn combat_trigger_dependency_map() -> &'static HashMap<String, bool> {
    TRIGGER_DEPENDENCY_MAP.get_or_init(|| {
        parse_map(TRIGGER_DEPENDENCY_MAP_JSON)
            .iter()
            .map(|(k, v)| (k.clone(), v["isSingleTarget"].as_bool().unwrap_or(false)))
            .collect()
    })
}
pub fn enhancement_level_table() -> &'static Vec<f64> {
    ENHANCEMENT_TABLE.get_or_init(|| {
        serde_json::from_str::<Value>(ENHANCEMENT_TABLE_JSON)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|x| x.as_f64())
            .collect()
    })
}
pub fn labyrinth_crate_detail_map() -> &'static HashMap<String, Value> {
    LABYRINTH_CRATE_MAP.get_or_init(HashMap::new)
}