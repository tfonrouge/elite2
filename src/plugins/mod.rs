//! Game systems, each implemented as a self-contained Bevy [`Plugin`].
//!
//! Architecture convention (see CLAUDE.md):
//! - **Components** hold data, **systems** hold behavior, **resources** hold
//!   global state.
//! - Each major game system is one plugin in its own module. `main` composes
//!   the plugins; plugins do not reach into each other's internals.
//!
//! As the roadmap progresses this module gains a sibling per system
//! (`flight`, `combat`, `galaxy`, `economy`, `station`, ...).
//!
//! [`Plugin`]: bevy::prelude::Plugin

pub mod camera;
pub mod core;
pub mod world;
