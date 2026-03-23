use rand::{Rng, RngExt};

/// A weighted entry in a population table.
#[derive(Clone, Debug)]
pub struct PopEntry<T> {
    pub value: T,
    pub weight: f32,
}

/// A population table: weighted random selection with support for
/// pick-one, pick-each, and nested sub-tables.
///
/// Based on the Caves of Qud pattern — a simple, expressive tool for
/// discrete procedural decisions.
#[derive(Clone, Debug)]
pub enum PopTable<T: Clone> {
    /// Pick one entry at random, weighted.
    PickOne(Vec<PopEntry<T>>),
    /// Pick each entry independently (each has a probability = weight/100).
    PickEach(Vec<PopEntry<T>>),
    /// Pick a fixed number of entries at random, weighted, without replacement.
    PickN(Vec<PopEntry<T>>, u32),
}

impl<T: Clone> PopTable<T> {
    /// Create a pick-one table from (value, weight) pairs.
    pub fn pick_one(entries: Vec<(T, f32)>) -> Self {
        Self::PickOne(entries.into_iter().map(|(v, w)| PopEntry { value: v, weight: w }).collect())
    }

    /// Create a pick-each table from (value, probability%) pairs.
    /// Weight is treated as percentage chance (0-100).
    pub fn pick_each(entries: Vec<(T, f32)>) -> Self {
        Self::PickEach(entries.into_iter().map(|(v, w)| PopEntry { value: v, weight: w }).collect())
    }

    /// Create a pick-N table from (value, weight) pairs.
    pub fn pick_n(entries: Vec<(T, f32)>, n: u32) -> Self {
        Self::PickN(entries.into_iter().map(|(v, w)| PopEntry { value: v, weight: w }).collect(), n)
    }

    /// Roll on this table and return the selected values.
    pub fn roll(&self, rng: &mut impl Rng) -> Vec<T> {
        match self {
            Self::PickOne(entries) => {
                if let Some(picked) = weighted_pick(entries, rng) {
                    vec![picked]
                } else {
                    vec![]
                }
            }
            Self::PickEach(entries) => {
                let mut results = Vec::new();
                for entry in entries {
                    let chance = (entry.weight / 100.0).clamp(0.0, 1.0);
                    if rng.random::<f32>() < chance {
                        results.push(entry.value.clone());
                    }
                }
                results
            }
            Self::PickN(entries, n) => {
                let mut results = Vec::new();
                let mut remaining: Vec<&PopEntry<T>> = entries.iter().collect();
                for _ in 0..*n {
                    if remaining.is_empty() {
                        break;
                    }
                    let total: f32 = remaining.iter().map(|e| e.weight).sum();
                    if total <= 0.0 {
                        break;
                    }
                    let mut roll = rng.random::<f32>() * total;
                    let mut picked_idx = 0;
                    for (i, entry) in remaining.iter().enumerate() {
                        roll -= entry.weight;
                        if roll <= 0.0 {
                            picked_idx = i;
                            break;
                        }
                    }
                    results.push(remaining[picked_idx].value.clone());
                    remaining.remove(picked_idx);
                }
                results
            }
        }
    }

    /// Roll and return exactly one value, or None if the table is empty.
    pub fn roll_one(&self, rng: &mut impl Rng) -> Option<T> {
        match self {
            Self::PickOne(entries) => weighted_pick(entries, rng),
            _ => self.roll(rng).into_iter().next(),
        }
    }
}

/// Weighted random selection from a slice of entries.
fn weighted_pick<T: Clone>(entries: &[PopEntry<T>], rng: &mut impl Rng) -> Option<T> {
    if entries.is_empty() {
        return None;
    }

    let total: f32 = entries.iter().map(|e| e.weight).sum();
    if total <= 0.0 {
        return None;
    }

    let mut roll = rng.random::<f32>() * total;
    for entry in entries {
        roll -= entry.weight;
        if roll <= 0.0 {
            return Some(entry.value.clone());
        }
    }

    Some(entries.last().unwrap().value.clone())
}

/// Convenience: build a pick-one table from a slice of (value, weight).
/// Useful for inline table construction.
pub fn pick_one<T: Clone>(entries: &[(T, f32)]) -> PopTable<T> {
    PopTable::pick_one(entries.to_vec())
}

pub fn pick_each<T: Clone>(entries: &[(T, f32)]) -> PopTable<T> {
    PopTable::pick_each(entries.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn rng() -> rand::rngs::StdRng {
        rand::rngs::StdRng::seed_from_u64(42)
    }

    #[test]
    fn pick_one_returns_single_value() {
        let table = PopTable::pick_one(vec![
            ("sword", 50.0),
            ("axe", 30.0),
            ("mace", 20.0),
        ]);
        let result = table.roll(&mut rng());
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn pick_one_respects_weights() {
        let table = PopTable::pick_one(vec![
            ("common", 1000.0),
            ("rare", 0.001),
        ]);
        // With such extreme weights, should almost always pick "common"
        let mut counts = [0u32; 2];
        let mut rng = rng();
        for _ in 0..100 {
            let result = table.roll_one(&mut rng).unwrap();
            if result == "common" { counts[0] += 1; } else { counts[1] += 1; }
        }
        assert!(counts[0] > 90);
    }

    #[test]
    fn pick_each_can_return_multiple() {
        let table = PopTable::pick_each(vec![
            ("a", 100.0), // 100% chance
            ("b", 100.0), // 100% chance
            ("c", 100.0), // 100% chance
        ]);
        let result = table.roll(&mut rng());
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn pick_each_can_return_none() {
        let table = PopTable::pick_each(vec![
            ("a", 0.0), // 0% chance
            ("b", 0.0),
        ]);
        let result = table.roll(&mut rng());
        assert!(result.is_empty());
    }

    #[test]
    fn pick_n_returns_n_unique() {
        let table = PopTable::pick_n(vec![
            ("a", 10.0),
            ("b", 10.0),
            ("c", 10.0),
            ("d", 10.0),
        ], 3);
        let result = table.roll(&mut rng());
        assert_eq!(result.len(), 3);
        // All unique
        let mut sorted = result.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn pick_n_caps_at_table_size() {
        let table = PopTable::pick_n(vec![
            ("a", 10.0),
            ("b", 10.0),
        ], 5);
        let result = table.roll(&mut rng());
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn empty_table_returns_empty() {
        let table: PopTable<&str> = PopTable::pick_one(vec![]);
        assert!(table.roll(&mut rng()).is_empty());
        assert!(table.roll_one(&mut rng()).is_none());
    }

    #[test]
    fn deterministic_with_same_seed() {
        let table = PopTable::pick_one(vec![
            ("a", 10.0), ("b", 10.0), ("c", 10.0),
        ]);
        let r1 = table.roll_one(&mut rng());
        let r2 = table.roll_one(&mut rng());
        assert_eq!(r1, r2);
    }
}
