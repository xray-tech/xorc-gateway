use proto_events;
use events;
use chrono::offset::Utc;

#[derive(Deserialize, Debug)]
pub struct SDKEventBatch
{
    pub environment: events::SDKEnvironment,
    pub events: Vec<events::SDKEvent>,
    pub device: events::SDKDevice,
}

impl Into<proto_events::SDKEventBatch> for SDKEventBatch {
    fn into(self) -> proto_events::SDKEventBatch {
        let mut sdk_event = proto_events::SDKEventBatch::new();
        sdk_event.set_environment(self.environment.clone().into());
        sdk_event.set_device(self.device.clone().into());

        {
            let header = sdk_event.mut_header();

            header.set_created_at(Utc::now().timestamp_millis());
            header.set_source(self.environment.app_id);
            header.set_field_type("events.SDKEventBatch".to_string());
            header.set_feed("360dialog".to_string());
        }


        let events = self.events.into_iter().map(|ev| ev.into()).collect();
        sdk_event.set_event(events);

        sdk_event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use proto_events;
    use serde_json;

    #[test]
    fn test_empty_header_values() {
        let json = json!({
            "environment": {},
            "device": {},
            "events": []
        });

        let device: SDKEventBatch = serde_json::from_value(json).unwrap();
        let proto: proto_events::SDKEventBatch = device.into();
        let header = proto.get_header();

        assert!(header.has_created_at());
        assert_eq!("", header.get_source());
        assert_eq!("events.SDKEventBatch", header.get_field_type());
        assert_eq!("360dialog", header.get_feed());
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
        let proto: proto_events::SDKEventBatch = device.into();
        let header = proto.get_header();

        assert_eq!("420", header.get_source());
    }
}
