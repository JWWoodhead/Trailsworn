use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

/// All named input actions in the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    // Camera
    CameraPanUp,
    CameraPanDown,
    CameraPanLeft,
    CameraPanRight,

    // Game speed
    Pause,
    Speed1,
    Speed2,
    Speed3,

    // Selection
    Select,
    Command,

    // Abilities
    AbilitySlot1,
    AbilitySlot2,
    AbilitySlot3,
    AbilitySlot4,
    AbilitySlot5,
    AbilitySlot6,

    // Escape / cancel
    Cancel,

    // UI panels
    ToggleCharacterSheet,
    ToggleInventory,

    // World map
    ToggleWorldMap,

    // Party selection (F1-F4)
    SelectPartyMember1,
    SelectPartyMember2,
    SelectPartyMember3,
    SelectPartyMember4,

    // Debug (Ctrl+F1-F6)
    DebugGrid,
    DebugPathing,
    DebugAggro,
    DebugAiState,
    DebugProfiling,
    DebugObstacles,
}

/// A raw input that can trigger an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputBinding {
    Key(KeyCode),
    /// Key that requires Ctrl to be held.
    CtrlKey(KeyCode),
    Mouse(MouseButton),
}

/// Maps raw inputs to actions. The single source of truth for all keybindings.
#[derive(Resource)]
pub struct InputMap {
    bindings: HashMap<InputBinding, Action>,
}

impl Default for InputMap {
    fn default() -> Self {
        let mut map = Self {
            bindings: HashMap::new(),
        };

        // Camera
        map.bind(InputBinding::Key(KeyCode::KeyW), Action::CameraPanUp);
        map.bind(InputBinding::Key(KeyCode::ArrowUp), Action::CameraPanUp);
        map.bind(InputBinding::Key(KeyCode::KeyS), Action::CameraPanDown);
        map.bind(InputBinding::Key(KeyCode::ArrowDown), Action::CameraPanDown);
        map.bind(InputBinding::Key(KeyCode::KeyA), Action::CameraPanLeft);
        map.bind(InputBinding::Key(KeyCode::ArrowLeft), Action::CameraPanLeft);
        map.bind(InputBinding::Key(KeyCode::KeyD), Action::CameraPanRight);
        map.bind(InputBinding::Key(KeyCode::ArrowRight), Action::CameraPanRight);

        // Game speed
        map.bind(InputBinding::Key(KeyCode::Space), Action::Pause);
        map.bind(InputBinding::Key(KeyCode::Digit1), Action::Speed1);
        map.bind(InputBinding::Key(KeyCode::Digit2), Action::Speed2);
        map.bind(InputBinding::Key(KeyCode::Digit3), Action::Speed3);

        // Selection / commands
        map.bind(InputBinding::Mouse(MouseButton::Left), Action::Select);
        map.bind(InputBinding::Mouse(MouseButton::Right), Action::Command);

        // Abilities
        map.bind(InputBinding::Key(KeyCode::KeyQ), Action::AbilitySlot1);
        map.bind(InputBinding::Key(KeyCode::KeyE), Action::AbilitySlot2);
        map.bind(InputBinding::Key(KeyCode::KeyR), Action::AbilitySlot3);
        map.bind(InputBinding::Key(KeyCode::KeyT), Action::AbilitySlot4);
        map.bind(InputBinding::Key(KeyCode::KeyF), Action::AbilitySlot5);
        map.bind(InputBinding::Key(KeyCode::KeyG), Action::AbilitySlot6);

        // Cancel
        map.bind(InputBinding::Key(KeyCode::Escape), Action::Cancel);

        // UI panels
        map.bind(InputBinding::Key(KeyCode::KeyC), Action::ToggleCharacterSheet);
        map.bind(InputBinding::Key(KeyCode::KeyI), Action::ToggleInventory);

        // World map
        map.bind(InputBinding::Key(KeyCode::KeyM), Action::ToggleWorldMap);

        // Party selection
        map.bind(InputBinding::Key(KeyCode::F1), Action::SelectPartyMember1);
        map.bind(InputBinding::Key(KeyCode::F2), Action::SelectPartyMember2);
        map.bind(InputBinding::Key(KeyCode::F3), Action::SelectPartyMember3);
        map.bind(InputBinding::Key(KeyCode::F4), Action::SelectPartyMember4);

        // Debug (Ctrl+F key)
        map.bind(InputBinding::CtrlKey(KeyCode::F1), Action::DebugGrid);
        map.bind(InputBinding::CtrlKey(KeyCode::F2), Action::DebugPathing);
        map.bind(InputBinding::CtrlKey(KeyCode::F3), Action::DebugAggro);
        map.bind(InputBinding::CtrlKey(KeyCode::F4), Action::DebugAiState);
        map.bind(InputBinding::CtrlKey(KeyCode::F5), Action::DebugProfiling);
        map.bind(InputBinding::CtrlKey(KeyCode::F6), Action::DebugObstacles);

        map
    }
}

impl InputMap {
    pub fn bind(&mut self, input: InputBinding, action: Action) {
        self.bindings.insert(input, action);
    }

    pub fn action_for(&self, input: &InputBinding) -> Option<Action> {
        self.bindings.get(input).copied()
    }
}

/// The set of actions active this frame. Populated by `process_input`.
/// Systems read this instead of raw keyboard/mouse state.
#[derive(Resource, Default)]
pub struct ActionState {
    pressed: HashSet<Action>,
    just_pressed: HashSet<Action>,
    just_released: HashSet<Action>,
}

impl ActionState {
    pub fn pressed(&self, action: Action) -> bool {
        self.pressed.contains(&action)
    }

    pub fn just_pressed(&self, action: Action) -> bool {
        self.just_pressed.contains(&action)
    }

    pub fn just_released(&self, action: Action) -> bool {
        self.just_released.contains(&action)
    }

    fn clear(&mut self) {
        self.pressed.clear();
        self.just_pressed.clear();
        self.just_released.clear();
    }
}

/// Single system that reads raw input and populates ActionState.
/// Must run before all other systems that read actions.
pub fn process_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    input_map: Res<InputMap>,
    mut actions: ResMut<ActionState>,
) {
    actions.clear();

    let ctrl_held = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

    for (binding, action) in &input_map.bindings {
        match binding {
            InputBinding::Key(key) => {
                // Plain key bindings only fire when Ctrl is NOT held,
                // so Ctrl+F1 doesn't also trigger a plain F1 binding.
                if ctrl_held { continue; }
                if keyboard.pressed(*key) {
                    actions.pressed.insert(*action);
                }
                if keyboard.just_pressed(*key) {
                    actions.just_pressed.insert(*action);
                }
                if keyboard.just_released(*key) {
                    actions.just_released.insert(*action);
                }
            }
            InputBinding::CtrlKey(key) => {
                if !ctrl_held { continue; }
                if keyboard.pressed(*key) {
                    actions.pressed.insert(*action);
                }
                if keyboard.just_pressed(*key) {
                    actions.just_pressed.insert(*action);
                }
                if keyboard.just_released(*key) {
                    actions.just_released.insert(*action);
                }
            }
            InputBinding::Mouse(button) => {
                if mouse.pressed(*button) {
                    actions.pressed.insert(*action);
                }
                if mouse.just_pressed(*button) {
                    actions.just_pressed.insert(*action);
                }
                if mouse.just_released(*button) {
                    actions.just_released.insert(*action);
                }
            }
        }
    }
}
