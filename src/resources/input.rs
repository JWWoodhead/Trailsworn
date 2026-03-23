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

    // Debug
    DebugGrid,
    DebugPathing,
    DebugAggro,
    DebugAiState,
}

/// A raw input that can trigger an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputBinding {
    Key(KeyCode),
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

        // Debug
        map.bind(InputBinding::Key(KeyCode::F1), Action::DebugGrid);
        map.bind(InputBinding::Key(KeyCode::F2), Action::DebugPathing);
        map.bind(InputBinding::Key(KeyCode::F3), Action::DebugAggro);
        map.bind(InputBinding::Key(KeyCode::F4), Action::DebugAiState);

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

    for (binding, action) in &input_map.bindings {
        match binding {
            InputBinding::Key(key) => {
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
