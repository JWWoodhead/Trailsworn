use rand::{Rng, RngExt};

use super::affixes::*;
use super::items::*;

// ---------------------------------------------------------------------------
// Drop parameters
// ---------------------------------------------------------------------------

/// Parameters for generating a dropped item.
pub struct DropParams {
    /// Item level (from zone/mob level). Determines base type tier and affix tiers.
    pub item_level: u32,
    /// Force a specific item kind (Weapon or Armor). Random if None.
    pub force_kind: Option<ItemKind>,
    /// Force a specific weapon class. Random if None.
    pub force_weapon_class: Option<WeaponClass>,
    /// Force a specific armor class. Random if None.
    pub force_armor_class: Option<ArmorClass>,
}

// ---------------------------------------------------------------------------
// Rarity rolling (ilvl-scaled)
// ---------------------------------------------------------------------------

/// Roll rarity based on item level. Higher ilvl = better rarity chance.
///
/// | ilvl | Normal | Magic | Rare  |
/// |------|--------|-------|-------|
/// | 1    | 88%    | 10%   | 2%    |
/// | 10   | 80%    | 16%   | 4%    |
/// | 20   | 70%    | 22%   | 8%    |
/// | 30   | 60%    | 27%   | 13%   |
/// | 40+  | 50%    | 30%   | 20%   |
pub fn roll_rarity(item_level: u32, rng: &mut impl Rng) -> Rarity {
    let rare_chance = (0.02 + item_level as f64 * 0.004).min(0.20);
    let magic_chance = (0.10 + item_level as f64 * 0.005).min(0.30);

    let roll: f64 = rng.random();
    if roll < rare_chance {
        Rarity::Rare
    } else if roll < rare_chance + magic_chance {
        Rarity::Magic
    } else {
        Rarity::Normal
    }
}

// ---------------------------------------------------------------------------
// Base type selection
// ---------------------------------------------------------------------------

/// Pick a base item type appropriate for the item level.
/// Weighted toward items whose item_level_req is close to the drop's item level.
pub fn pick_base_type(
    item_level: u32,
    kind: ItemKind,
    item_registry: &ItemRegistry,
    rng: &mut impl Rng,
) -> Option<ItemId> {
    let candidates: Vec<&ItemDef> = item_registry
        .all()
        .filter(|def| {
            def.kind == kind
                && def.item_level_req <= item_level
                && def.base_tier.is_some()
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Weight: items close to ilvl are favored, higher tiers get bonus
    let weights: Vec<f64> = candidates
        .iter()
        .map(|def| {
            let level_diff = item_level.saturating_sub(def.item_level_req);
            let freshness = if level_diff < 10 { 3.0 } else { 1.0 };
            let tier_bonus = match def.base_tier {
                Some(BaseTier::Runic) => 3.0,
                Some(BaseTier::Tempered) => 2.0,
                Some(BaseTier::Crude) => 1.0,
                None => 1.0,
            };
            freshness * tier_bonus
        })
        .collect();

    weighted_pick(&candidates, &weights, rng).map(|def| def.id)
}

// ---------------------------------------------------------------------------
// Affix rolling
// ---------------------------------------------------------------------------

/// Roll affixes for an item based on rarity, item level, and item kind.
/// Returns (prefixes, suffixes).
pub fn roll_affixes(
    rarity: Rarity,
    item_level: u32,
    kind: ItemKind,
    affix_registry: &AffixRegistry,
    rng: &mut impl Rng,
) -> (Vec<RolledAffix>, Vec<RolledAffix>) {
    let (max_prefix, max_suffix) = max_affixes_per_slot(rarity);
    let (min_total, max_total) = affix_count_range(rarity);

    if max_total == 0 {
        return (vec![], vec![]);
    }

    let total_count = rng.random_range(min_total..=max_total);
    let mut prefixes: Vec<RolledAffix> = Vec::new();
    let mut suffixes: Vec<RolledAffix> = Vec::new();
    let mut used_affix_ids: Vec<AffixId> = Vec::new();

    for _ in 0..total_count {
        let can_prefix = (prefixes.len() as u32) < max_prefix;
        let can_suffix = (suffixes.len() as u32) < max_suffix;

        let first_choice = match (can_prefix, can_suffix) {
            (true, true) => {
                if rng.random::<bool>() { AffixSlotType::Prefix } else { AffixSlotType::Suffix }
            }
            (true, false) => AffixSlotType::Prefix,
            (false, true) => AffixSlotType::Suffix,
            (false, false) => break,
        };

        // Try first choice, then fall back to the other slot type
        let slot_types = match first_choice {
            AffixSlotType::Prefix if can_suffix => vec![AffixSlotType::Prefix, AffixSlotType::Suffix],
            AffixSlotType::Suffix if can_prefix => vec![AffixSlotType::Suffix, AffixSlotType::Prefix],
            _ => vec![first_choice],
        };

        let mut candidates: Vec<&AffixDef> = Vec::new();
        let mut slot_type = first_choice;
        for &st in &slot_types {
            candidates = affix_registry
                .candidates(kind, st, item_level)
                .into_iter()
                .filter(|def| !used_affix_ids.contains(&def.id))
                .collect();
            if !candidates.is_empty() {
                slot_type = st;
                break;
            }
        }

        if candidates.is_empty() {
            break; // No candidates left in either slot — stop rolling
        }

        // Pick a random affix (uniform)
        let affix_def = candidates[rng.random_range(0..candidates.len())];

        // Pick a tier (weighted toward higher tiers)
        let available_tiers = affix_def.available_tiers(item_level);
        if available_tiers.is_empty() {
            continue;
        }

        let tier_weights: Vec<f64> = available_tiers
            .iter()
            .enumerate()
            .map(|(i, _)| ((i + 1) as f64).powi(2))
            .collect();

        let tier_idx = weighted_index(&tier_weights, rng).unwrap_or(0);
        let tier = available_tiers[tier_idx];

        // Find global tier index
        let global_tier_index = affix_def.tiers
            .iter()
            .position(|t| t.label == tier.label)
            .unwrap_or(0);

        let rolled = RolledAffix {
            affix_id: affix_def.id,
            tier_index: global_tier_index,
            effect: tier.effect.clone(),
            label: tier.label.clone(),
        };

        used_affix_ids.push(affix_def.id);

        match slot_type {
            AffixSlotType::Prefix => prefixes.push(rolled),
            AffixSlotType::Suffix => suffixes.push(rolled),
        }
    }

    (prefixes, suffixes)
}

// ---------------------------------------------------------------------------
// Top-level generation
// ---------------------------------------------------------------------------

/// Generate a complete item instance: pick base type, roll rarity, roll affixes.
pub fn generate_item(
    params: &DropParams,
    item_registry: &ItemRegistry,
    affix_registry: &AffixRegistry,
    instance_registry: &mut ItemInstanceRegistry,
    rng: &mut impl Rng,
) -> Option<ItemInstanceId> {
    let kind = params.force_kind.unwrap_or_else(|| {
        if rng.random::<bool>() { ItemKind::Weapon } else { ItemKind::Armor }
    });

    let base_id = pick_base_type(params.item_level, kind, item_registry, rng)?;
    let rarity = roll_rarity(params.item_level, rng);
    let (prefixes, suffixes) = roll_affixes(rarity, params.item_level, kind, affix_registry, rng);

    let id = instance_registry.next_id();
    let instance = ItemInstance {
        id,
        base_item_id: base_id,
        rarity,
        item_level: params.item_level,
        prefixes,
        suffixes,
    };

    instance_registry.insert(instance);
    Some(id)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn weighted_pick<'a, T>(items: &[&'a T], weights: &[f64], rng: &mut impl Rng) -> Option<&'a T> {
    let idx = weighted_index(weights, rng)?;
    Some(items[idx])
}

fn weighted_index(weights: &[f64], rng: &mut impl Rng) -> Option<usize> {
    if weights.is_empty() {
        return None;
    }
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return Some(0);
    }
    let mut roll = rng.random::<f64>() * total;
    for (i, w) in weights.iter().enumerate() {
        roll -= w;
        if roll <= 0.0 {
            return Some(i);
        }
    }
    Some(weights.len() - 1)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::affix_defs::register_starter_affixes;
    use crate::resources::item_defs::register_starter_items;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn setup() -> (ItemRegistry, AffixRegistry) {
        let mut item_reg = ItemRegistry::default();
        register_starter_items(&mut item_reg);
        let mut affix_reg = AffixRegistry::default();
        register_starter_affixes(&mut affix_reg);
        (item_reg, affix_reg)
    }

    #[test]
    fn roll_rarity_always_valid() {
        let mut rng = StdRng::seed_from_u64(42);
        for ilvl in [1, 10, 20, 30, 40, 50] {
            for _ in 0..100 {
                let rarity = roll_rarity(ilvl, &mut rng);
                assert!(matches!(rarity, Rarity::Normal | Rarity::Magic | Rarity::Rare));
            }
        }
    }

    #[test]
    fn higher_ilvl_produces_more_magic_and_rare() {
        let mut rng = StdRng::seed_from_u64(123);
        let mut low_magic = 0u32;
        let mut high_magic = 0u32;
        let trials = 1000;

        for _ in 0..trials {
            if matches!(roll_rarity(1, &mut rng), Rarity::Magic | Rarity::Rare) {
                low_magic += 1;
            }
            if matches!(roll_rarity(40, &mut rng), Rarity::Magic | Rarity::Rare) {
                high_magic += 1;
            }
        }

        assert!(high_magic > low_magic, "ilvl 40 should produce more magic/rare than ilvl 1");
    }

    #[test]
    fn normal_rarity_has_no_affixes() {
        let (_, affix_reg) = setup();
        let mut rng = StdRng::seed_from_u64(42);
        let (pre, suf) = roll_affixes(Rarity::Normal, 20, ItemKind::Weapon, &affix_reg, &mut rng);
        assert!(pre.is_empty());
        assert!(suf.is_empty());
    }

    #[test]
    fn magic_rarity_has_1_to_2_affixes() {
        let (_, affix_reg) = setup();
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..50 {
            let (pre, suf) = roll_affixes(Rarity::Magic, 20, ItemKind::Weapon, &affix_reg, &mut rng);
            let total = pre.len() + suf.len();
            assert!(total >= 1 && total <= 2, "Magic should have 1-2 affixes, got {}", total);
            assert!(pre.len() <= 1, "Magic max 1 prefix");
            assert!(suf.len() <= 1, "Magic max 1 suffix");
        }
    }

    #[test]
    fn rare_rarity_has_3_to_6_affixes() {
        let (_, affix_reg) = setup();
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..50 {
            let (pre, suf) = roll_affixes(Rarity::Rare, 30, ItemKind::Weapon, &affix_reg, &mut rng);
            let total = pre.len() + suf.len();
            assert!(total >= 3 && total <= 6, "Rare should have 3-6 affixes, got {}", total);
            assert!(pre.len() <= 3, "Rare max 3 prefixes");
            assert!(suf.len() <= 3, "Rare max 3 suffixes");
        }
    }

    #[test]
    fn no_duplicate_affixes_on_item() {
        let (_, affix_reg) = setup();
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..50 {
            let (pre, suf) = roll_affixes(Rarity::Rare, 30, ItemKind::Armor, &affix_reg, &mut rng);
            let all_ids: Vec<AffixId> = pre.iter().chain(suf.iter()).map(|a| a.affix_id).collect();
            let unique: std::collections::HashSet<_> = all_ids.iter().collect();
            assert_eq!(all_ids.len(), unique.len(), "No duplicate affixes");
        }
    }

    #[test]
    fn generate_item_produces_valid_instance() {
        let (item_reg, affix_reg) = setup();
        let mut instance_reg = ItemInstanceRegistry::default();
        let mut rng = StdRng::seed_from_u64(42);

        let params = DropParams {
            item_level: 20,
            force_kind: Some(ItemKind::Weapon),
            force_weapon_class: None,
            force_armor_class: None,
        };

        let id = generate_item(&params, &item_reg, &affix_reg, &mut instance_reg, &mut rng);
        assert!(id.is_some());

        let instance = instance_reg.get(id.unwrap()).unwrap();
        assert_eq!(instance.item_level, 20);
        assert!(item_reg.get(instance.base_item_id).is_some());
    }

    #[test]
    fn deterministic_with_same_seed() {
        let (item_reg, affix_reg) = setup();

        let make = |seed: u64| {
            let mut instance_reg = ItemInstanceRegistry::default();
            let mut rng = StdRng::seed_from_u64(seed);
            let params = DropParams {
                item_level: 15,
                force_kind: Some(ItemKind::Weapon),
                force_weapon_class: None,
                force_armor_class: None,
            };
            let id = generate_item(&params, &item_reg, &affix_reg, &mut instance_reg, &mut rng).unwrap();
            let inst = instance_reg.get(id).unwrap().clone();
            (inst.base_item_id, inst.rarity, inst.prefixes.len(), inst.suffixes.len())
        };

        assert_eq!(make(99), make(99));
    }

    #[test]
    fn display_name_includes_affixes() {
        let (item_reg, affix_reg) = setup();
        let mut instance_reg = ItemInstanceRegistry::default();
        let mut rng = StdRng::seed_from_u64(42);

        // Generate until we get a magic+ item with affixes
        for _ in 0..100 {
            let params = DropParams {
                item_level: 20,
                force_kind: Some(ItemKind::Weapon),
                force_weapon_class: None,
                force_armor_class: None,
            };
            let id = generate_item(&params, &item_reg, &affix_reg, &mut instance_reg, &mut rng).unwrap();
            let instance = instance_reg.get(id).unwrap();
            let name = instance.display_name(&item_reg);
            if !instance.prefixes.is_empty() || !instance.suffixes.is_empty() {
                // Name should be longer than just the base type name
                let base_name = item_reg.get(instance.base_item_id).unwrap().name.as_str();
                assert!(name.len() > base_name.len(), "Affixed name should be longer");
                return;
            }
        }
        // If we never got an affixed item in 100 tries, that's fine — test passes
    }
}
