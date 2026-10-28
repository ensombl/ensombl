use crate::application::ports::BuildInfo;

/// Build facts baked in at compile time from Cargo metadata.
pub struct CargoBuildInfo;

impl BuildInfo for CargoBuildInfo {
    fn name(&self) -> &str {
        "CRM"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
}
