use bevy::prelude::*;
use std::collections::HashMap;

/// Index into a body's part list.
pub type PartIndex = usize;

/// What a body part enables when functional.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    Sight,
    Hearing,
    Manipulation,
    Movement,
    Breathing,
    Consciousness,
    Eating,
}

/// Definition of a single body part within a template.
#[derive(Clone, Debug)]
pub struct BodyPartDef {
    pub name: String,
    pub parent: Option<PartIndex>,
    pub max_hp: f32,
    /// Probability weight of being hit (higher = more likely).
    pub coverage: f32,
    /// If destroyed, the character dies.
    pub vital: bool,
    /// What capabilities this part provides.
    pub capabilities: Vec<Capability>,
}

/// A reusable body layout. Loaded from data, shared across characters of the same type.
#[derive(Clone, Debug)]
pub struct BodyTemplate {
    pub name: String,
    pub parts: Vec<BodyPartDef>,
}

impl BodyTemplate {
    /// Get all child indices for a given part.
    pub fn children_of(&self, index: PartIndex) -> Vec<PartIndex> {
        self.parts
            .iter()
            .enumerate()
            .filter(|(_, p)| p.parent == Some(index))
            .map(|(i, _)| i)
            .collect()
    }

    /// Get all descendant indices (recursive children) for a given part.
    pub fn descendants_of(&self, index: PartIndex) -> Vec<PartIndex> {
        let mut result = Vec::new();
        let mut stack = self.children_of(index);
        while let Some(i) = stack.pop() {
            result.push(i);
            stack.extend(self.children_of(i));
        }
        result
    }
}

/// Runtime state of a single body part on a specific character.
#[derive(Clone, Debug)]
pub struct BodyPartState {
    pub current_hp: f32,
    pub destroyed: bool,
}

/// A character's body — references a template and holds per-part runtime state.
#[derive(Component, Clone, Debug)]
pub struct Body {
    pub template_id: String,
    pub parts: Vec<BodyPartState>,
}

impl Body {
    /// Create a new body from a template with all parts at full HP.
    pub fn from_template(template: &BodyTemplate) -> Self {
        let parts = template
            .parts
            .iter()
            .map(|def| BodyPartState {
                current_hp: def.max_hp,
                destroyed: false,
            })
            .collect();

        Self {
            template_id: template.name.clone(),
            parts,
        }
    }

    /// Apply damage to a specific part. Returns actual damage dealt.
    /// If the part is destroyed, marks it and all children as destroyed.
    pub fn damage_part(
        &mut self,
        index: PartIndex,
        damage: f32,
        template: &BodyTemplate,
    ) -> f32 {
        let state = &mut self.parts[index];
        if state.destroyed {
            return 0.0;
        }

        let actual = damage.min(state.current_hp);
        state.current_hp -= actual;

        if state.current_hp <= 0.0 {
            state.destroyed = true;
            // Destroy all children
            for child in template.descendants_of(index) {
                self.parts[child].destroyed = true;
                self.parts[child].current_hp = 0.0;
            }
        }

        actual
    }

    /// Heal a specific body part. Cannot heal destroyed parts. Returns actual healing done.
    pub fn heal_part(
        &mut self,
        index: PartIndex,
        amount: f32,
        template: &BodyTemplate,
    ) -> f32 {
        let state = &mut self.parts[index];
        if state.destroyed {
            return 0.0;
        }
        let max_hp = template.parts[index].max_hp;
        let actual = amount.min(max_hp - state.current_hp);
        state.current_hp += actual;
        actual
    }

    /// Distribute healing across all damaged (non-destroyed) body parts.
    /// Returns total healing done.
    pub fn heal_distributed(
        &mut self,
        total_amount: f32,
        template: &BodyTemplate,
    ) -> f32 {
        // Find all damaged, non-destroyed parts
        let damaged: Vec<usize> = self
            .parts
            .iter()
            .enumerate()
            .filter(|(i, state)| {
                !state.destroyed && state.current_hp < template.parts[*i].max_hp
            })
            .map(|(i, _)| i)
            .collect();

        if damaged.is_empty() {
            return 0.0;
        }

        let per_part = total_amount / damaged.len() as f32;
        let mut total_healed = 0.0;
        for idx in damaged {
            total_healed += self.heal_part(idx, per_part, template);
        }
        total_healed
    }

    /// Check if any vital part is destroyed.
    pub fn is_dead(&self, template: &BodyTemplate) -> bool {
        template
            .parts
            .iter()
            .enumerate()
            .any(|(i, def)| def.vital && self.parts[i].destroyed)
    }

    /// Get total pain level (0.0 = no pain, 1.0+ = extreme).
    /// Based on ratio of damage across all parts.
    pub fn pain_level(&self, template: &BodyTemplate) -> f32 {
        let mut total_damage = 0.0;
        let mut total_max = 0.0;
        for (i, def) in template.parts.iter().enumerate() {
            total_max += def.max_hp;
            total_damage += def.max_hp - self.parts[i].current_hp;
        }
        if total_max <= 0.0 {
            return 0.0;
        }
        total_damage / total_max
    }

    /// Check if a capability is still functional (at least one part providing it is intact).
    pub fn has_capability(&self, capability: &Capability, template: &BodyTemplate) -> bool {
        template.parts.iter().enumerate().any(|(i, def)| {
            !self.parts[i].destroyed && def.capabilities.contains(capability)
        })
    }

    /// Get the fraction of a capability remaining (e.g., 0.5 if one of two eyes is destroyed).
    pub fn capability_fraction(&self, capability: &Capability, template: &BodyTemplate) -> f32 {
        let providers: Vec<_> = template
            .parts
            .iter()
            .enumerate()
            .filter(|(_, def)| def.capabilities.contains(capability))
            .collect();

        if providers.is_empty() {
            return 0.0;
        }

        let functional = providers
            .iter()
            .filter(|(i, _)| !self.parts[*i].destroyed)
            .count();

        functional as f32 / providers.len() as f32
    }
}

/// Registry of all body templates, keyed by name.
#[derive(Resource, Default)]
pub struct BodyTemplates {
    pub templates: HashMap<String, BodyTemplate>,
}

impl BodyTemplates {
    pub fn register(&mut self, template: BodyTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    pub fn get(&self, name: &str) -> Option<&BodyTemplate> {
        self.templates.get(name)
    }
}

/// Build the standard humanoid body template.
pub fn humanoid_template() -> BodyTemplate {
    use Capability::*;

    let parts = vec![
        // 0: Head
        BodyPartDef {
            name: "Head".into(),
            parent: None,
            max_hp: 25.0,
            coverage: 0.1,
            vital: false,
            capabilities: vec![],
        },
        // 1: Brain
        BodyPartDef {
            name: "Brain".into(),
            parent: Some(0),
            max_hp: 10.0,
            coverage: 0.02,
            vital: true,
            capabilities: vec![Consciousness],
        },
        // 2: Left Eye
        BodyPartDef {
            name: "Left Eye".into(),
            parent: Some(0),
            max_hp: 8.0,
            coverage: 0.02,
            vital: false,
            capabilities: vec![Sight],
        },
        // 3: Right Eye
        BodyPartDef {
            name: "Right Eye".into(),
            parent: Some(0),
            max_hp: 8.0,
            coverage: 0.02,
            vital: false,
            capabilities: vec![Sight],
        },
        // 4: Jaw
        BodyPartDef {
            name: "Jaw".into(),
            parent: Some(0),
            max_hp: 12.0,
            coverage: 0.03,
            vital: false,
            capabilities: vec![Eating],
        },
        // 5: Torso
        BodyPartDef {
            name: "Torso".into(),
            parent: None,
            max_hp: 40.0,
            coverage: 0.30,
            vital: false,
            capabilities: vec![],
        },
        // 6: Heart
        BodyPartDef {
            name: "Heart".into(),
            parent: Some(5),
            max_hp: 12.0,
            coverage: 0.02,
            vital: true,
            capabilities: vec![],
        },
        // 7: Left Lung
        BodyPartDef {
            name: "Left Lung".into(),
            parent: Some(5),
            max_hp: 12.0,
            coverage: 0.03,
            vital: false,
            capabilities: vec![Breathing],
        },
        // 8: Right Lung
        BodyPartDef {
            name: "Right Lung".into(),
            parent: Some(5),
            max_hp: 12.0,
            coverage: 0.03,
            vital: false,
            capabilities: vec![Breathing],
        },
        // 9: Stomach
        BodyPartDef {
            name: "Stomach".into(),
            parent: Some(5),
            max_hp: 15.0,
            coverage: 0.03,
            vital: false,
            capabilities: vec![Eating],
        },
        // 10: Left Arm
        BodyPartDef {
            name: "Left Arm".into(),
            parent: Some(5),
            max_hp: 25.0,
            coverage: 0.08,
            vital: false,
            capabilities: vec![],
        },
        // 11: Left Hand
        BodyPartDef {
            name: "Left Hand".into(),
            parent: Some(10),
            max_hp: 15.0,
            coverage: 0.04,
            vital: false,
            capabilities: vec![Manipulation],
        },
        // 12: Right Arm
        BodyPartDef {
            name: "Right Arm".into(),
            parent: Some(5),
            max_hp: 25.0,
            coverage: 0.08,
            vital: false,
            capabilities: vec![],
        },
        // 13: Right Hand
        BodyPartDef {
            name: "Right Hand".into(),
            parent: Some(12),
            max_hp: 15.0,
            coverage: 0.04,
            vital: false,
            capabilities: vec![Manipulation],
        },
        // 14: Left Leg
        BodyPartDef {
            name: "Left Leg".into(),
            parent: None,
            max_hp: 30.0,
            coverage: 0.08,
            vital: false,
            capabilities: vec![],
        },
        // 15: Left Foot
        BodyPartDef {
            name: "Left Foot".into(),
            parent: Some(14),
            max_hp: 15.0,
            coverage: 0.04,
            vital: false,
            capabilities: vec![Movement],
        },
        // 16: Right Leg
        BodyPartDef {
            name: "Right Leg".into(),
            parent: None,
            max_hp: 30.0,
            coverage: 0.08,
            vital: false,
            capabilities: vec![],
        },
        // 17: Right Foot
        BodyPartDef {
            name: "Right Foot".into(),
            parent: Some(16),
            max_hp: 15.0,
            coverage: 0.04,
            vital: false,
            capabilities: vec![Movement],
        },
    ];

    BodyTemplate {
        name: "humanoid".into(),
        parts,
    }
}
