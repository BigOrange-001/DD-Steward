//! Parsers for the original game data. Rule numbers live only in the game's
//! files; this crate reads them and never hard-codes values.

pub mod ai;
pub mod blood011;
pub mod bonus;
pub mod build010;
pub mod candidate012;
pub mod closure012;
pub mod combat;
pub mod content;
pub mod contract011;
pub mod darkest;
pub mod difficulty012;
pub mod dson013;
pub mod encounter011;
pub mod execution012;
pub mod farm011;
pub mod hero010;
pub mod host_rules012;
pub mod initial013;
pub mod items;
pub mod library;
pub mod lifecycle;
pub mod lighting;
pub mod mechanics;
pub mod mode_effects012;
pub mod nightmare011;
pub mod overlay;
pub mod rules_inventory012;
mod schema012;
pub mod special;
pub mod support;

pub mod trinket010;

use std::path::PathBuf;

/// Env var that overrides the game directory.
pub const GAME_DIR_ENV: &str = "DD_GAME_DIR";

/// The Darkest Dungeon install directory.
///
/// Uses `DD_GAME_DIR` if set, otherwise the packaged sibling game-data fixture.
pub fn game_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(GAME_DIR_ENV) {
        return PathBuf::from(dir);
    }
    default_game_dir()
}
pub fn default_game_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("game-data")
}

pub mod curios013;

pub mod catalog014;
pub mod fixed014;
pub mod generation014;
pub mod maps014;

pub mod inventory015;
pub mod opening015;

pub mod opening_spawn015;

pub mod town016;

pub mod campaign017;

pub mod field017;

pub mod camp017;

pub mod quest_items017;

pub mod retention017;

pub mod content019;
pub mod objectives020;
