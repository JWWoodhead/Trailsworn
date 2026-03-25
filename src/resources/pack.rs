use bevy::prelude::*;

/// Identifies which pack an enemy belongs to. Enemies sharing the same `PackId`
/// will aggro together when any member is threatened.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackId(pub u32);
