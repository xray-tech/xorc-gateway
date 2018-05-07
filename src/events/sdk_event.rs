use proto_events;
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

impl Into<proto_events::SDKEventData> for SDKEvent {
    fn into(self) -> proto_events::SDKEventData {
        let mut ev = proto_events::SDKEventData::new();

        {
            let properties = ev.mut_properties();

            for property in self.properties().into_iter() {
                properties.push(property);
            }
        }

        ev.set_id(self.id);

        if let Some(session_id) = self.session_id {
            ev.set_session_id(session_id);
        }
        if let Some(timestamp) = self.timestamp {
            ev.set_timestamp(timestamp);
        }
        if let Some(name) = self.name {
            ev.set_name(name);
        }
        if let Some(external_user_id) = self.external_user_id {
            ev.set_external_user_id(external_user_id);
        }
        if let Some(reference_id) = self.reference_id {
            ev.set_reference_id(reference_id);
        }

        ev
    }
}

impl SDKEvent
{
    fn properties(&self) -> Vec<proto_events::SDKEventData_Property> {
        let mut properties = Vec::new();
        Self::flatten_properties("", &self.properties, &mut properties);

        properties
    }

    fn flatten_properties(
        prefix: &str,
        properties: &Map<String, Value>,
        mut container: &mut Vec<proto_events::SDKEventData_Property>,)
    {
        for (key, value) in properties.iter() {
            let prefixed_key = format!("{}{}", prefix, key);

            match value {
                &Value::String(ref s) => {
                    let mut ev = proto_events::SDKEventData_Property::new();
                    ev.set_key(prefixed_key);
                    ev.set_string_value(s.to_string());
                    container.push(ev);
                },
                &Value::Bool(b) => {
                    let mut ev = proto_events::SDKEventData_Property::new();
                    ev.set_key(prefixed_key);
                    ev.set_bool_value(b);
                    container.push(ev);
                },
                &Value::Number(ref n) => {
                    let p_value = if let Some(i) = n.as_i64() {
                        i as f64
                    } else if let Some(i) = n.as_u64() {
                        i as f64
                    } else {
                        n.as_f64().unwrap()
                    };

                    let mut ev = proto_events::SDKEventData_Property::new();
                    ev.set_key(prefixed_key);
                    ev.set_number_value(p_value);
                    container.push(ev);
                },
                &Value::Object(ref map) => {
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

    use proto_events;
    use serde_json;

    #[test]
    fn test_with_empty_properties() {
        let json = json!({
            "properties": {}
        });

        let event: SDKEvent = serde_json::from_value(json).unwrap();
        let proto: proto_events::SDKEventData = event.into();

        assert!(proto.get_properties().is_empty());
    }

    #[test]
    fn test_with_string_property() {
        let json = json!({
            "properties": {
                "foo": "bar",
            }
        });

        let event: SDKEvent = serde_json::from_value(json).unwrap();
        let proto: proto_events::SDKEventData = event.into();

        assert_eq!("foo", proto.get_properties()[0].get_key());
        assert_eq!("bar", proto.get_properties()[0].get_string_value());
    }

    #[test]
    fn test_with_number_property() {
        let json = json!({
            "properties": {
                "foo": 420,
            }
        });

        let event: SDKEvent = serde_json::from_value(json).unwrap();
        let proto: proto_events::SDKEventData = event.into();

        assert_eq!("foo", proto.get_properties()[0].get_key());
        assert_eq!(420.0, proto.get_properties()[0].get_number_value());
    }

    #[test]
    fn test_with_bool_property() {
        let json = json!({
            "properties": {
                "foo": true,
            }
        });

        let event: SDKEvent = serde_json::from_value(json).unwrap();
        let proto: proto_events::SDKEventData = event.into();

        assert_eq!("foo", proto.get_properties()[0].get_key());
        assert_eq!(true, proto.get_properties()[0].get_bool_value());
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
        let proto: proto_events::SDKEventData = event.into();

        assert_eq!("foo__bar", proto.get_properties()[0].get_key());
        assert_eq!("lol", proto.get_properties()[0].get_string_value());
    }
}
