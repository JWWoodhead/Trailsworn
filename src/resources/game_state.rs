use bevy::prelude::*;

/// Top-level game state. Controls which systems run and entity cleanup on transition.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    /// Asset loading, setup.
    #[default]
    Loading,
    /// Main gameplay.
    Playing,
}

/// System sets for coarse ordering within the Update schedule.
/// Configure ordering centrally; individual systems just use `.in_set(GameSet::X)`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Player input handling (always responsive, even during pause).
    Input,
    /// Game time advancement, tick accumulation.
    Tick,
    /// AI evaluation, task execution, and movement pathfinding.
    Ai,
    /// Combat resolution: attacks, damage, status effects.
    Combat,
    /// Entity movement along paths.
    Movement,
    /// UI updates: health bars, floating text, tooltips.
    Ui,
    /// Visual sync: transforms, rendering.
    Render,
}
