use events::output;

#[derive(Deserialize, Debug, Clone)]
pub struct SDKEnvironment {
    pub sdk_version: Option<String>,
    pub app_version: Option<String>,
    pub app_store_id: Option<String>,
    #[serde(default)] pub app_id: String,
    pub app_instance_id: Option<String>,
}

impl Into<output::events::SdkEnvironment> for SDKEnvironment {
    fn into(self) -> output::events::SdkEnvironment {
        output::events::SdkEnvironment {
            app_id: Some(self.app_id),
            sdk_version: self.sdk_version,
            app_version: self.app_version,
            app_store_id: self.app_store_id,
            app_instance_id: self.app_instance_id,
        }
    }
}
