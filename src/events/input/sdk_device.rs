use events::output::common;

#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Ios,
    Android,
    Web,
    Unknown
}

impl<'a> From<&'a Platform> for String {
    fn from(platform: &'a Platform) -> String {
        match platform {
            Platform::Ios => "ios".to_string(),
            Platform::Android => "android".to_string(),
            Platform::Web => "web".to_string(),
            _ => String::new()
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SDKDevice
{
    #[serde(default)] ifa_tracking_enabled: bool,
    #[serde(default)] notification_registered: bool,
    h: Option<i32>,
    w: Option<i32>,
    locale: Option<String>,
    language: Option<String>,
    time_zone: Option<String>,
    manufacturer: Option<String>,
    model: Option<String>,
    os_version: Option<String>,
    os_name: Option<String>,
    network_connection_type: Option<String>,
    device_name: Option<String>,
    ifa: Option<String>,
    idfv: Option<String>,
    carrier_name: Option<String>,
    carrier_country: Option<String>,
    browser_name: Option<String>,
    browser_version: Option<String>,
    browser_ua: Option<String>,
    notification_types: Option<i32>,
    orientation: Option<String>,
    platform: Option<String>,
}

impl SDKDevice
{
    pub fn platform(&self) -> Platform
    {
        match self.platform.as_ref().map(|x| &**x) {
            Some("android") => Platform::Android,
            Some("ios")     => Platform::Ios,
            Some("web")     => Platform::Web,

            _ => match self.os_name.as_ref().map(|x| &**x) {
                Some("iOS") | Some("iPhone OS") => Platform::Ios,
                Some("Android")                 => Platform::Android,
                _                               => Platform::Unknown
            }
        }
    }
}

impl Into<common::Device> for SDKDevice {
    fn into(self) -> common::Device {
        let mut dev = common::Device {
            platform: Some(String::from(&self.platform())),
            locale: self.locale,
            timezone: self.time_zone,
            manufacturer: self.manufacturer,
            model: self.model,
            osv: self.os_version,
            os: self.os_name,
            connectiontype: self.network_connection_type,
            ifa: self.ifa,
            idfv: self.idfv,
            notification_types: self.notification_types,
            orientation: self.orientation,
            h: self.h.or(Some(-1)),
            w: self.w.or(Some(-1)),
            ifa_tracking_enabled: Some(self.ifa_tracking_enabled),
            notification_registered: Some(self.notification_registered),

            carrier: Some(common::Carrier {
                name: self.carrier_name,
                mcc: self.carrier_country,
                ..Default::default()
            }),

            browser: Some(common::Browser {
                name: self.browser_name,
                ua: self.browser_ua,
                version: self.browser_version,
                ..Default::default()
            }),

            ..Default::default()
        };

        if let Some(language) = self.language {
            dev.language = Some(language);
        } else if let Some(ref locale) = dev.locale {
            dev.language = locale
                .find("_")
                .map(|index| {
                    locale[0..index].to_string()
                });
        }

        dev
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use events::output::common;
    use serde_json;

    #[test]
    fn test_no_language_or_local_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: common::Device = device.into();

        assert!(proto.language.is_none());
    }

    #[test]
    fn test_no_language_or_locale_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: common::Device = device.into();

        assert!(proto.language.is_none());
    }

    #[test]
    fn test_language_set() {
        let json = json!({
            "language": "fi"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: common::Device = device.into();

        assert_eq!(
            Some(String::from("fi")),
            proto.language
        );
    }

    #[test]
    fn test_locale_set() {
        let json = json!({
            "locale": "fi_FI"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: common::Device = device.into();

        assert_eq!(
            Some(String::from("fi")),
            proto.language
        );
    }

    #[test]
    fn test_broken_locale_set() {
        let json = json!({
            "locale": "kulli"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: common::Device = device.into();

        assert!(proto.language.is_none());
    }

    #[test]
    fn test_no_h_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: common::Device = device.into();

        assert_eq!(Some(-1), proto.h);
    }

    #[test]
    fn test_h_set() {
        let json = json!({
            "h": 420
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: common::Device = device.into();

        assert_eq!(Some(420), proto.h);
    }

    #[test]
    fn test_no_w_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: common::Device = device.into();

        assert_eq!(Some(-1), proto.w);
    }

    #[test]
    fn test_w_set() {
        let json = json!({
            "w": 715517
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: common::Device = device.into();

        assert_eq!(Some(715517), proto.w);
    }

    #[test]
    fn test_no_ifa_tracking_enabled_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: common::Device = device.into();

        assert_eq!(Some(false), proto.ifa_tracking_enabled);
    }

    #[test]
    fn test_no_notification_registered_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: common::Device = device.into();

        assert_eq!(Some(false), proto.notification_registered);
    }

    #[test]
    fn test_no_os_name_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: common::Device = device.into();

        assert_eq!(Some(String::new()), proto.platform);
    }

    #[test]
    fn test_os_name_ios() {
        let json = json!({
            "os_name": "iOS"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: common::Device = device.into();

        assert_eq!(Some(String::from("ios")), proto.platform);
    }

    #[test]
    fn test_os_name_iphone_os() {
        let json = json!({
            "os_name": "iPhone OS"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: common::Device = device.into();

        assert_eq!(Some(String::from("ios")), proto.platform);
    }

    #[test]
    fn test_os_name_android() {
        let json = json!({
            "os_name": "Android"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: common::Device = device.into();

        assert_eq!(Some(String::from("android")), proto.platform);
    }

    #[test]
    fn test_platform_web() {
        let json = json!({
            "platform": "web"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: common::Device = device.into();

        assert_eq!(Some(String::from("web")), proto.platform);
    }
}
