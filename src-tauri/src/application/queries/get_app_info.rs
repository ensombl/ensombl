use std::sync::Arc;

use serde::Serialize;
use ts_rs::TS;

use crate::application::cqrs::{Query, QueryHandler};
use crate::application::ports::BuildInfo;
use crate::application::Result;

pub struct GetAppInfo;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

impl Query for GetAppInfo {
    type Output = AppInfo;
}

pub struct GetAppInfoHandler {
    build: Arc<dyn BuildInfo>,
}

impl GetAppInfoHandler {
    pub fn new(build: Arc<dyn BuildInfo>) -> Self {
        Self { build }
    }
}

impl QueryHandler<GetAppInfo> for GetAppInfoHandler {
    fn handle(&self, _: GetAppInfo) -> Result<AppInfo> {
        Ok(AppInfo {
            name: self.build.name().into(),
            version: self.build.version().into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeBuild;
    impl BuildInfo for FakeBuild {
        fn name(&self) -> &str {
            "CRM"
        }
        fn version(&self) -> &str {
            "9.9.9"
        }
    }

    #[test]
    fn returns_build_info() {
        let handler = GetAppInfoHandler::new(Arc::new(FakeBuild));
        let info = handler.handle(GetAppInfo).unwrap();
        assert_eq!(info, AppInfo { name: "CRM".into(), version: "9.9.9".into() });
    }
}
