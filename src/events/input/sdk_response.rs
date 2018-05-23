use encryption::Ciphertext;

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Success,
}

#[derive(Serialize, Debug)]
pub struct RegistrationData {
    api_token: Option<String>,
    device_id: Ciphertext,
}

#[derive(Serialize, Debug)]
pub struct SDKResponse {
    events_status: Vec<EventResult>
}

impl From<Vec<EventResult>> for SDKResponse {
    fn from(events_status: Vec<EventResult>) -> SDKResponse {
        SDKResponse {
            events_status
        }
    }
}

#[derive(Serialize, Debug)]
pub struct EventResult {
    pub id: String,
    pub registration_data: Option<RegistrationData>,
    pub status: EventStatus,
}

impl EventResult {
    pub fn new(id: String, status: EventStatus) -> EventResult {
        EventResult {
            id: id,
            registration_data: None,
            status: status,
        }
    }

    pub fn register(
        id: String,
        status: EventStatus,
        api_token: Option<String>,
        ciphertext: Ciphertext,
    ) -> EventResult
    {
        let registration_data = Some(RegistrationData {
            api_token: api_token,
            device_id: ciphertext,
        });

        EventResult {
            id,
            status,
            registration_data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encryption::*;
    use serde_json;

    #[test]
    fn test_register_event_result() {
        let token = Some(String::from("token"));
        let cipher = Ciphertext::from("encrypted");
        let event_result = EventResult::register(
            "123".to_string(),
            EventStatus::Success,
            token.clone(),
            cipher.clone(),
        );

        let sdk_response = SDKResponse::from(vec!(event_result));

        let json_expected = json!({
            "events_status": [
                {
                    "id": "123",
                    "status": "success",
                    "registration_data": {
                        "device_id": "encrypted",
                        "api_token": "token"
                    },
                }
            ]
        });

        assert_eq!(
            serde_json::to_string(&json_expected).unwrap(),
            serde_json::to_string(&sdk_response).unwrap()
        );
    }
}
