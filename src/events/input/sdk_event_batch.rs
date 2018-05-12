use events::{input, output};
use chrono::offset::Utc;

#[derive(Deserialize, Debug)]
pub struct SDKEventBatch
{
    pub environment: input::SDKEnvironment,
    pub events: Vec<input::SDKEvent>,
    pub device: input::SDKDevice,
}

impl Into<output::events::SdkEventBatch> for SDKEventBatch {
    fn into(self) -> output::events::SdkEventBatch {
        output::events::SdkEventBatch {
            header: output::common::Header {
                created_at: Utc::now().timestamp_millis(),
                source: self.environment.app_id.clone(),
                type_: Some(String::from("events.SDKEventBatch")),
                feed: Some(String::from("360dialog")),
                ..Default::default()
            },
            environment: Some(self.environment.into()),
            device: Some(self.device.into()),
            event: self.events.into_iter().map(|ev| ev.into()).collect(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use events::output;
    use serde_json;

    #[test]
    fn test_empty_header_values() {
        let json = json!({
            "environment": {},
            "device": {},
            "events": []
        });

        let device: SDKEventBatch = serde_json::from_value(json).unwrap();
        let proto: output::events::SdkEventBatch = device.into();
        let header = proto.header;

        assert_eq!(header.source, String::new());
        assert_eq!(
            Some(String::from("events.SDKEventBatch")),
            header.type_
        );
        assert_eq!(Some(String::from("360dialog")), header.feed);
    }

    #[test]
    fn test_with_environment_app_id() {
        let json = json!({
            "environment": {
                "app_id": "420",
            },
            "device": {},
            "events": []
        });

        let device: SDKEventBatch = serde_json::from_value(json).unwrap();
        let proto: output::events::SdkEventBatch = device.into();
        let header = proto.header;

        assert_eq!(String::from("420"), header.source);
    }
}
