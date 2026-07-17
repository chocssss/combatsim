/// All event types used in the combat simulation event queue.
/// We use an enum to avoid heap allocation for every event.

use crate::combatsimulator::ability::Ability;

/// Unique string tags for each event type (mirrors JS static type fields)
pub const COMBAT_START: &str = "combatStart";
pub const PLAYER_RESPAWN: &str = "playerRespawn";
pub const ENEMY_RESPAWN: &str = "enemyRespawn";
pub const AUTO_ATTACK: &str = "autoAttack";
pub const CONSUMABLE_TICK: &str = "consumableTick";
pub const DAMAGE_OVER_TIME: &str = "damageOverTime";
pub const CHECK_BUFF_EXPIRATION: &str = "checkBuffExpiration";
pub const REGEN_TICK: &str = "regenTick";
pub const STUN_EXPIRATION: &str = "stunExpiration";
pub const BLIND_EXPIRATION: &str = "blindExpiration";
pub const SILENCE_EXPIRATION: &str = "silenceExpiration";
pub const CURSE_EXPIRATION: &str = "curseExpiration";
pub const WEAKEN_EXPIRATION: &str = "weakenExpiration";
pub const FURY_EXPIRATION: &str = "furyExpiration";
pub const ENRAGE_TICK: &str = "enrageTick";
pub const ABILITY_CAST_END: &str = "abilityCastEndEvent";
pub const AWAIT_COOLDOWN: &str = "awaitCooldownEvent";
pub const COOLDOWN_READY: &str = "cooldownReady";

/// Index into the global unit arena (CombatSimulator.all_units)
pub type UnitIdx = usize;

#[derive(Clone, Debug)]
pub struct CombatEvent {
    pub time: i64,
    pub kind: EventKind,
}

#[derive(Clone, Debug)]
pub enum EventKind {
    CombatStart,
    PlayerRespawn { hrid: String },
    EnemyRespawn,
    AutoAttack { source: UnitIdx },
    ConsumableTick { source: UnitIdx, consumable_hrid: String, total_ticks: i32, current_tick: i32 },
    DamageOverTime { source_ref: UnitIdx, target: UnitIdx, damage: f64, total_ticks: i32, current_tick: i32, combat_style_hrid: String },
    CheckBuffExpiration { source: UnitIdx },
    RegenTick,
    StunExpiration { source: UnitIdx },
    BlindExpiration { source: UnitIdx },
    SilenceExpiration { source: UnitIdx },
    CurseExpiration { source: UnitIdx, curse_amount: i32 },
    WeakenExpiration { source: UnitIdx, weaken_amount: i32 },
    FuryExpiration { source: UnitIdx, fury_amount: f64 },
    EnrageTick { encounter_time: i64 },
    AbilityCastEnd { source: UnitIdx, ability_idx: usize },
    AwaitCooldown { source: UnitIdx },
    CooldownReady,
}

impl CombatEvent {
    pub fn type_str(&self) -> &'static str {
        match &self.kind {
            EventKind::CombatStart => COMBAT_START,
            EventKind::PlayerRespawn { .. } => PLAYER_RESPAWN,
            EventKind::EnemyRespawn => ENEMY_RESPAWN,
            EventKind::AutoAttack { .. } => AUTO_ATTACK,
            EventKind::ConsumableTick { .. } => CONSUMABLE_TICK,
            EventKind::DamageOverTime { .. } => DAMAGE_OVER_TIME,
            EventKind::CheckBuffExpiration { .. } => CHECK_BUFF_EXPIRATION,
            EventKind::RegenTick => REGEN_TICK,
            EventKind::StunExpiration { .. } => STUN_EXPIRATION,
            EventKind::BlindExpiration { .. } => BLIND_EXPIRATION,
            EventKind::SilenceExpiration { .. } => SILENCE_EXPIRATION,
            EventKind::CurseExpiration { .. } => CURSE_EXPIRATION,
            EventKind::WeakenExpiration { .. } => WEAKEN_EXPIRATION,
            EventKind::FuryExpiration { .. } => FURY_EXPIRATION,
            EventKind::EnrageTick { .. } => ENRAGE_TICK,
            EventKind::AbilityCastEnd { .. } => ABILITY_CAST_END,
            EventKind::AwaitCooldown { .. } => AWAIT_COOLDOWN,
            EventKind::CooldownReady => COOLDOWN_READY,
        }
    }

    pub fn source_idx(&self) -> Option<UnitIdx> {
        match &self.kind {
            EventKind::AutoAttack { source }
            | EventKind::ConsumableTick { source, .. }
            | EventKind::CheckBuffExpiration { source }
            | EventKind::StunExpiration { source }
            | EventKind::BlindExpiration { source }
            | EventKind::SilenceExpiration { source }
            | EventKind::CurseExpiration { source, .. }
            | EventKind::WeakenExpiration { source, .. }
            | EventKind::FuryExpiration { source, .. }
            | EventKind::AbilityCastEnd { source, .. }
            | EventKind::AwaitCooldown { source } => Some(*source),
            EventKind::DamageOverTime { source_ref, .. } => Some(*source_ref),
            _ => None,
        }
    }

    pub fn target_idx(&self) -> Option<UnitIdx> {
        match &self.kind {
            EventKind::DamageOverTime { target, .. } => Some(*target),
            _ => None,
        }
    }
}
