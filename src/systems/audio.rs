use bevy::prelude::*;

use crate::resources::audio::AudioAssets;

/// Load audio assets into the AudioAssets resource.
/// Currently a stub — no .ogg files exist yet. When audio files are added to
/// `assets/audio/`, load them here and insert into the map.
pub fn setup_audio(mut commands: Commands) {
    // No audio files yet — insert an empty resource.
    // When assets are added, use asset_server.load("audio/melee_hit.ogg") etc.
    commands.insert_resource(AudioAssets::default());
}
