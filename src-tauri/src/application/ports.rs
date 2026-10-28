//! Ports: traits owned by the application layer and implemented by
//! infrastructure adapters.

/// Static facts about the running build.
pub trait BuildInfo: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
}
