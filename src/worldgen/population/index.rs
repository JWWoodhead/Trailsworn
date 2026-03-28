//! Settlement-to-residents index for fast per-settlement lookups.

use std::collections::HashMap;

use super::types::Person;

/// Maps settlement IDs to indices into the `Vec<Person>`.
/// Rebuilt once per year — cheaper than incremental maintenance.
pub struct SettlementIndex {
    map: HashMap<u32, Vec<usize>>,
}

impl SettlementIndex {
    /// Build index from living people only.
    pub fn build(people: &[Person], year: i32) -> Self {
        let mut map: HashMap<u32, Vec<usize>> = HashMap::new();
        for (i, person) in people.iter().enumerate() {
            if person.is_alive(year) {
                map.entry(person.settlement_id).or_default().push(i);
            }
        }
        Self { map }
    }

    /// Get resident indices for a settlement.
    pub fn residents(&self, settlement_id: u32) -> &[usize] {
        self.map.get(&settlement_id).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
