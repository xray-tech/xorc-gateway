use proto_events;

#[derive(Deserialize, Debug, Clone)]
pub struct SDKEnvironment {
    pub sdk_version: Option<String>,
    pub app_version: Option<String>,
    pub app_store_id: Option<String>,
    #[serde(default)]
    pub app_id: String,
    pub app_instance_id: Option<String>,
}

impl Into<proto_events::SDKEnvironment> for SDKEnvironment {
    fn into(self) -> proto_events::SDKEnvironment {
        let mut env = proto_events::SDKEnvironment::new();

        env.set_app_id(self.app_id);

        if let Some(sdk_version) = self.sdk_version {
            env.set_sdk_version(sdk_version)
        };
        if let Some(app_version) = self.app_version {
            env.set_app_version(app_version)
        };
        if let Some(app_store_id) = self.app_store_id {
            env.set_app_store_id(app_store_id)
        };
        if let Some(app_instance_id) = self.app_instance_id {
            env.set_app_instance_id(app_instance_id)
        };

        env
    }
}
