use proto_events;
use events;
use chrono::offset::Utc;

#[derive(Deserialize, Debug)]
pub struct SDKEventBatch
{
    environment: events::SDKEnvironment,
    events: Vec<events::SDKEvent>,
    device: events::SDKDevice,
}

impl Into<proto_events::SDKEventBatch> for SDKEventBatch {
    fn into(self) -> proto_events::SDKEventBatch {
        let mut sdk_event = proto_events::SDKEventBatch::new();

        {
            let header = sdk_event.mut_header();
            header.set_created_at(Utc::now().timestamp_millis());

            if let Some(ref app_id) = self.environment.app_id {
                header.set_source(app_id.to_string());
            }

            header.set_field_type("events.SDKEvent".to_string());
            header.set_feed("360dialog".to_string());
        }

        sdk_event.set_environment(self.environment.clone().into());
        sdk_event.set_device(self.device.clone().into());

        let events = self.events.into_iter().map(|ev| ev.into()).collect();
        sdk_event.set_event(events);

        sdk_event
    }
}
