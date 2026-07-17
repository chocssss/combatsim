use std::collections::BinaryHeap;
use std::cmp::Reverse;
use crate::combatsimulator::events::{CombatEvent, EventKind, UnitIdx};

/// A min-heap event queue (smallest time first).
pub struct EventQueue {
    heap: BinaryHeap<Reverse<OrdEvent>>,
}

/// Wrapper to make CombatEvent orderable by time
#[derive(Clone, Debug)]
struct OrdEvent(CombatEvent, u64); // u64 = insertion order for stable sort

impl PartialEq for OrdEvent {
    fn eq(&self, other: &Self) -> bool {
        self.0.time == other.0.time && self.1 == other.1
    }
}
impl Eq for OrdEvent {}
impl PartialOrd for OrdEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrdEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.time.cmp(&other.0.time).then(self.1.cmp(&other.1))
    }
}

impl EventQueue {
    pub fn new() -> Self {
        EventQueue { heap: BinaryHeap::new() }
    }

    pub fn add_event(&mut self, event: CombatEvent) {
        let order = self.heap.len() as u64;
        self.heap.push(Reverse(OrdEvent(event, order)));
    }

    pub fn get_next_event(&mut self) -> Option<CombatEvent> {
        self.heap.pop().map(|Reverse(oe)| oe.0)
    }

    pub fn clear(&mut self) {
        self.heap.clear();
    }

    pub fn clear_events_for_unit(&mut self, unit: UnitIdx) {
        self.retain(|e| {
            // Remove if source or target is this unit
            let is_source = e.source_idx() == Some(unit);
            let is_target = e.target_idx() == Some(unit);
            is_source || is_target
        });
    }

    pub fn clear_events_of_type(&mut self, type_str: &str) {
        self.retain(|e| e.type_str() == type_str);
    }

    pub fn clear_matching<F>(&mut self, predicate: F) -> bool
    where F: Fn(&CombatEvent) -> bool
    {
        let before = self.heap.len();
        self.retain(|e| predicate(e));
        self.heap.len() < before
    }

    pub fn contains_event_of_type(&self, type_str: &str) -> bool {
        self.heap.iter().any(|Reverse(oe)| oe.0.type_str() == type_str)
    }

    pub fn contains_event_of_type_and_hrid(&self, type_str: &str, hrid: &str) -> bool {
        self.heap.iter().any(|Reverse(oe)| {
            oe.0.type_str() == type_str && matches!(&oe.0.kind, EventKind::PlayerRespawn { hrid: h } if h == hrid)
        })
    }

    pub fn get_matching<F>(&self, predicate: F) -> Option<&CombatEvent>
    where F: Fn(&CombatEvent) -> bool
    {
        self.heap.iter()
            .map(|Reverse(oe)| &oe.0)
            .find(|e| predicate(e))
    }

    /// Remove all events matching predicate (retain = remove those for which predicate returns true)
    fn retain<F>(&mut self, should_remove: F)
    where F: Fn(&CombatEvent) -> bool
    {
        let mut remaining: Vec<Reverse<OrdEvent>> = Vec::new();
        while let Some(item) = self.heap.pop() {
            if !should_remove(&item.0.0) {
                remaining.push(item);
            }
        }
        for item in remaining {
            self.heap.push(item);
        }
    }
}
