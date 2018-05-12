use headers::DeviceHeaders;

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Success,
}

#[derive(Serialize, Debug)]
pub struct RegistrationData<'a> {
    api_token: &'a Option<String>,
    device_id: &'a Option<String>,
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
        device_headers: &'a DeviceHeaders
    ) -> EventResult<'a>
    {
        let registration_data = Some(RegistrationData {
            device_id: &device_headers.device_id.ciphertext,
            api_token: &device_headers.api_token,
        });

        EventResult {
            id, status, registration_data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use headers::{DeviceHeaders, DeviceId};
    use serde_json;

    #[test]
    fn test_register_event_result() {
        let headers = DeviceHeaders {
            device_id: DeviceId  {
                ciphertext: Some("encrypted".to_string()),
                cleartext: Some("everybody to read".to_string()),
            },
            api_token: Some("token".to_string()),
            signature: Some("signature".to_string()),
            ip: Some("ip".to_string()),
            origin: None,
        };

        let event_result = EventResult::register("123", EventStatus::Success, &headers);
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
