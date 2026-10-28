//! Domain layer: entities, value objects and pure rules. No IO and no
//! framework dependencies. Types derive `Serialize`/`TS` so they can cross
//! the IPC boundary unchanged; that's a derive, not a dependency on Tauri.

pub mod model;
