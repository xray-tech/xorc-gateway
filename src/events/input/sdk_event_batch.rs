use events::{input, output};
use chrono::offset::Utc;
use context::Context;

#[derive(Deserialize, Debug)]
pub struct SDKEventBatch
{
    pub environment: input::SDKEnvironment,
    pub events: Vec<input::SDKEvent>,
    pub device: input::SDKDevice,
    pub recipient_id: Option<String>,
}

impl SDKEventBatch
{
    pub fn into_proto(mut self, context: &Context) -> output::events::SdkEventBatch {
        if let Some(ref device_id) = context.device_id {
            self.recipient_id = Some(device_id.cleartext.clone().into());
        }

        if let Some(ref ip) = context.ip {
            self.device.set_location(ip);
        }

        self.events.sort_unstable_by(|e1, e2| {
            e1.timestamp.cmp(&e2.timestamp)
        });

        output::events::SdkEventBatch {
            header: output::common::Header {
                created_at: Utc::now().timestamp_millis(),
                source: self.environment.app_id.clone(),
                type_: Some(String::from("events.SDKEventBatch")),
                feed: Some(String::from("360dialog")),
                recipient_id: self.recipient_id,
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
    use hyper::HeaderMap;
    use http::header::HeaderValue;
    use context::Context;
    use events::input::Platform;

    #[test]
    fn test_empty_header_values() {
        let json = json!({
            "environment": {},
            "device": {},
            "events": []
        });

        let mut header_map = HeaderMap::new();
        let ip = "109.68.226.154";
        let cipher = "PNslnKKJkbq8Nv5/C0CcoK7hnFsdltcW3yK/I0QYJ7bUX8EHx2/NX0r8OkJHC5lzY/cBwZ3FeeFmRRpxof+rtw==";

        header_map.insert(
            "x-real-ip",
            HeaderValue::from_static(ip),
        );

        header_map.insert(
            "D360-Device-Id",
            HeaderValue::from_static(cipher),
        );

        let context = Context::new(&header_map, "123", Platform::Ios);

        let batch: SDKEventBatch = serde_json::from_value(json).unwrap();
        let proto: output::events::SdkEventBatch = batch.into_proto(&context);
        let header = proto.header;
        let device = proto.device.unwrap();

        assert_eq!(header.source, String::new());
        assert_eq!(
            Some(String::from("events.SDKEventBatch")),
            header.type_
        );
        assert_eq!(
            Some(String::from("8f7f5c07-5eb2-4695-870c-065d886cdc9e")),
            header.recipient_id,
        );
        assert_eq!(
            Some(String::from("DE")),
            device.country,
        );
        assert_eq!(
            Some(String::from("KOx83wHFu0JCTyitEEfU0+J0GWN5OxtXgMeIzDUinonr8ya0IY5VyYtrbDu8tRlBSo/a1T70lQ3uYcnSRYiR8w==")),
            device.ip_hashed_blake2,
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

        let header_map = HeaderMap::new();
        let context = Context::new(&header_map, "123", Platform::Ios);

        let device: SDKEventBatch = serde_json::from_value(json).unwrap();
        let proto: output::events::SdkEventBatch = device.into_proto(&context);
        let header = proto.header;
        let device = proto.device.unwrap();

        assert_eq!(String::from("420"), header.source);
        assert!(device.country.is_none());
        assert!(device.ip_hashed_blake2.is_none());
        assert!(header.recipient_id.is_none());
    }
}
