use proto_events;
use serde_json::{
    value::Value,
    Map,
};

#[derive(Deserialize, Debug)]
pub struct SDKEvent
{
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub timestamp: Option<String>,
    pub name: Option<String>,
    pub external_user_id: Option<String>,
    properties: Map<String, Value>,
    pub is_in_control_group: Option<i32>,
    pub reference_id: Option<String>,
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

        if let Some(id) = self.id {
            ev.set_id(id);
        }
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
    pub fn properties(&self) -> Vec<proto_events::SDKEventData_Property> {
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
                    println!("JSON object of type {:?} not supported", other);
                }
            };
        }
    }
}

