//! The divine simulation: god definitions, personality, territory, worship,
//! drive-based actions, divine conflict, and flaw triggers.

pub mod gods;
pub mod personality;
pub mod state;
pub mod territory;
pub mod worship;
pub mod drives;
pub mod conflict;
pub mod flaws;
pub mod artifacts;
pub mod races;
pub mod creatures;
pub mod sites;
pub mod terrain_scars;

pub use gods::{GodId, GodPool, DrawnPantheon, build_god_pool};
pub use terrain_scars::DivineTerrainType;
