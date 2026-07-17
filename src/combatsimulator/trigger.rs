use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::combatsimulator::data;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trigger {
    pub dependency_hrid: String,
    pub condition_hrid: String,
    pub comparator_hrid: String,
    pub value: f64,
}

impl Trigger {
    pub fn new(dependency_hrid: String, condition_hrid: String, comparator_hrid: String, value: f64) -> Self {
        Trigger { dependency_hrid, condition_hrid, comparator_hrid, value }
    }

    pub fn from_dto(dto: &Value) -> Self {
        Trigger {
            dependency_hrid: dto["dependencyHrid"].as_str().unwrap_or("").to_string(),
            condition_hrid: dto["conditionHrid"].as_str().unwrap_or("").to_string(),
            comparator_hrid: dto["comparatorHrid"].as_str().unwrap_or("").to_string(),
            value: dto["value"].as_f64().unwrap_or(0.0),
        }
    }
}

/// Trigger evaluation is done in combat_simulator with access to unit data.
/// We keep helper functions here that take raw data rather than unit refs
/// to avoid circular references.

pub struct TriggerContext<'a> {
    pub dependency_hrid: &'a str,
    pub condition_hrid: &'a str,
    pub comparator_hrid: &'a str,
    pub value: f64,
}

impl<'a> TriggerContext<'a> {
    pub fn compare(&self, dependency_value: f64) -> bool {
        match self.comparator_hrid {
            "/combat_trigger_comparators/greater_than_equal" => dependency_value >= self.value,
            "/combat_trigger_comparators/less_than_equal" => dependency_value <= self.value,
            "/combat_trigger_comparators/is_active" => dependency_value != 0.0,
            "/combat_trigger_comparators/is_inactive" => dependency_value == 0.0,
            _ => false,
        }
    }

    pub fn is_single_target(&self) -> bool {
        *data::combat_trigger_dependency_map()
            .get(self.dependency_hrid)
            .unwrap_or(&false)
    }
}
