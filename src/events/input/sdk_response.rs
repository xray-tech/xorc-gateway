use encryption::Ciphertext;

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Success,
}

#[derive(Serialize, Debug)]
pub struct RegistrationData<'a> {
    api_token: &'a Option<String>,
    device_id: &'a Option<Ciphertext>,
}

#[derive(Serialize, Debug)]
pub struct SDKResponse<'a> {
    events_status: Vec<EventResult<'a>>
}

impl<'a> From<Vec<EventResult<'a>>> for SDKResponse<'a> {
    fn from(events_status: Vec<EventResult<'a>>) -> SDKResponse<'a> {
        SDKResponse {
            events_status
        }
    }
}

#[derive(Serialize, Debug)]
pub struct EventResult<'a> {
    pub id: &'a str,
    pub registration_data: Option<RegistrationData<'a>>,
    pub status: EventStatus,
}

impl<'a> EventResult<'a> {
    pub fn register(
        id: &'a str,
        status: EventStatus,
        api_token: &'a Option<String>,
        ciphertext: &'a Option<Ciphertext>
    ) -> EventResult<'a>
    {
        let registration_data = Some(RegistrationData {
            api_token: api_token,
            device_id: ciphertext,
        });

        EventResult {
            id, status, registration_data,
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
        let cipher = Some(Ciphertext::from("encrypted"));
        let event_result = EventResult::register(
            "123",
            EventStatus::Success,
            &token,
            &cipher,
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
