use events::output::{
    self,
    events::{
        sdk_event_data::Property,
        sdk_event_data::property
    }
};

use serde_json::{
    value::Value,
    Map,
};

#[derive(Deserialize, Debug)]
pub struct SDKEvent
{
    #[serde(default = "default_event_id")]
    pub id: String,

    #[serde(default)]
    properties: Map<String, Value>,

    pub session_id: Option<String>,
    pub timestamp: Option<String>,
    pub name: Option<String>,
    pub external_user_id: Option<String>,
    pub is_in_control_group: Option<i32>,
    pub reference_id: Option<String>,
}

fn default_event_id() -> String {
    "0".to_string()
}

impl Into<output::events::SdkEventData> for SDKEvent {
    fn into(self) -> output::events::SdkEventData {
        output::events::SdkEventData {
            properties: self.properties(),
            id: Some(self.id),
            session_id: self.session_id,
            timestamp: self.timestamp,
            name: self.name,
            external_user_id: self.external_user_id,
            reference_id: self.reference_id,
            ..Default::default()
        }
    }
}

impl SDKEvent
{
    fn properties(&self) -> Vec<Property> {
        let mut properties = Vec::new();
        Self::flatten_properties("", &self.properties, &mut properties);

        properties
    }

    fn flatten_properties(
        prefix: &str,
        properties: &Map<String, Value>,
        mut container: &mut Vec<Property>,)
    {
        for (key, value) in properties.iter() {
            let prefixed_key = format!("{}{}", prefix, key);

            match value {
                Value::String(s) => {
                    container.push(Property {
                        key: prefixed_key,
                        type_: Some(property::Type::StringValue(s.to_string()))
                    });
                },
                Value::Bool(b) => {
                    container.push(Property {
                        key: prefixed_key,
                        type_: Some(property::Type::BoolValue(*b))
                    });
                },
                Value::Number(n) => {
                    let p_value = if let Some(i) = n.as_i64() {
                        i as f64
                    } else if let Some(i) = n.as_u64() {
                        i as f64
                    } else {
                        n.as_f64().unwrap()
                    };

                    container.push(Property {
                        key: prefixed_key,
                        type_: Some(property::Type::NumberValue(p_value))
                    });
                },
                Value::Object(map) => {
                    let prefix = format!("{}{}__", prefix, prefixed_key);
                    Self::flatten_properties(&prefix, map, &mut container);
                },
                other => {
                    warn!("JSON object of type {:?} not supported", other);
                }
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use events::output::{
        self,
        events::{
            sdk_event_data::Property,
            sdk_event_data::property
        }
    };

    use serde_json;

    #[test]
    fn test_with_empty_properties() {
        let json = json!({
            "properties": {}
        });

        let event: SDKEvent = serde_json::from_value(json).unwrap();
        let proto: output::SdkEventData = event.into();

        assert!(proto.properties.is_empty());
    }

    #[test]
    fn test_with_string_property() {
        let json = json!({
            "properties": {
                "foo": "bar",
            }
        });

        let event: SDKEvent = serde_json::from_value(json).unwrap();
        let proto: output::SdkEventData = event.into();

        assert_eq!("foo", proto.properties[0].key);
        assert_eq!(
            Some(property::Type::StringValue(String::from("bar"))),
            proto.properties[0].type_
        );
    }

    #[test]
    fn test_with_number_property() {
        let json = json!({
            "properties": {
                "foo": 420,
            }
        });

        let event: SDKEvent = serde_json::from_value(json).unwrap();
        let proto: output::SdkEventData = event.into();

        assert_eq!("foo", proto.properties[0].key);
        assert_eq!(
            Some(property::Type::NumberValue(420.0)),
            proto.properties[0].type_
        );
    }

    #[test]
    fn test_with_bool_property() {
        let json = json!({
            "properties": {
                "foo": true,
            }
        });

        let event: SDKEvent = serde_json::from_value(json).unwrap();
        let proto: output::SdkEventData = event.into();

        assert_eq!("foo", proto.properties[0].key);
        assert_eq!(
            Some(property::Type::BoolValue(true)),
            proto.properties[0].type_
        );
    }

    #[test]
    fn test_with_object_property() {
        let json = json!({
            "properties": {
                "foo": {
                    "bar": "lol",
                },
            }
        });

        let event: SDKEvent = serde_json::from_value(json).unwrap();
        let proto: output::SdkEventData = event.into();

        assert_eq!("foo__bar", proto.properties[0].key);
        assert_eq!(
            Some(property::Type::StringValue(String::from("lol"))),
            proto.properties[0].type_,
        );
    }
}
