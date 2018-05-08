use proto_events;

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
            &Platform::Ios => "ios".to_string(),
            &Platform::Android => "android".to_string(),
            &Platform::Web => "web".to_string(),
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

impl Into<proto_events::Device> for SDKDevice {
    fn into(self) -> proto_events::Device {
        let mut device = proto_events::Device::new();

        device.set_platform(String::from(&self.platform()));

        {
            let carrier = device.mut_carrier();
            if let Some(ref carrier_name) = self.carrier_name {
                carrier.set_name(carrier_name.to_string());
            }
            if let Some(ref carrier_country) = self.carrier_country {
                carrier.set_mcc(carrier_country.to_string());
            }
        }

        {
            let browser = device.mut_browser();
            if let Some(browser_name) = self.browser_name {
                browser.set_name(browser_name);
            }
            if let Some(browser_ua) = self.browser_ua {
                browser.set_ua(browser_ua);
            }
            if let Some(browser_version) = self.browser_version {
                browser.set_version(browser_version);
            }
        }

        if let Some(language) = self.language {
            device.set_language(language);
        } else if let Some(ref locale) = self.locale {
            if let Some(index) = locale.find("_") {
                device.set_language(locale[0..index].to_string());
            };
        };

        if let Some(locale) = self.locale {
            device.set_locale(locale)
        };
        if let Some(time_zone) = self.time_zone {
            device.set_timezone(time_zone);
        }
        if let Some(manufacturer) = self.manufacturer {
            device.set_manufacturer(manufacturer);
        }
        if let Some(model) = self.model {
            device.set_model(model);
        }
        if let Some(osv) = self.os_version {
            device.set_osv(osv);
        }
        if let Some(os) = self.os_name {
            device.set_os(os);
        }
        if let Some(connectiontype) = self.network_connection_type {
            device.set_connectiontype(connectiontype);
        }
        if let Some(ifa) = self.ifa {
            device.set_ifa(ifa);
        }
        if let Some(idfv) = self.idfv {
            device.set_idfv(idfv);
        }
        if let Some(notification_types) = self.notification_types {
            device.set_notification_types(notification_types);
        }
        if let Some(orientation) = self.orientation {
            device.set_orientation(orientation);
        }
        if let Some(h) = self.h {
            device.set_h(h);
        } else {
            device.set_h(-1);
        }
        if let Some(w) = self.w {
            device.set_w(w);
        } else {
            device.set_w(-1);
        }

        device.set_ifa_tracking_enabled(self.ifa_tracking_enabled);
        device.set_notification_registered(self.notification_registered);

        device
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use proto_events;
    use serde_json;

    #[test]
    fn test_no_language_or_local_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: proto_events::Device = device.into();

        assert!(!proto.has_language());
    }

    #[test]
    fn test_no_language_or_locale_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: proto_events::Device = device.into();

        assert!(!proto.has_language());
    }

    #[test]
    fn test_language_set() {
        let json = json!({
            "language": "fi"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!("fi", proto.get_language());
    }

    #[test]
    fn test_locale_set() {
        let json = json!({
            "locale": "fi_FI"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!("fi", proto.get_language());
    }

    #[test]
    fn test_broken_locale_set() {
        let json = json!({
            "locale": "kulli"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: proto_events::Device = device.into();

        assert!(!proto.has_language());
    }

    #[test]
    fn test_no_h_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!(-1, proto.get_h());
    }

    #[test]
    fn test_h_set() {
        let json = json!({
            "h": 420
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!(420, proto.get_h());
    }

    #[test]
    fn test_no_w_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!(-1, proto.get_w());
    }

    #[test]
    fn test_w_set() {
        let json = json!({
            "w": 715517
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!(715517, proto.get_w());
    }

    #[test]
    fn test_no_ifa_tracking_enabled_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!(false, proto.get_ifa_tracking_enabled());
    }

    #[test]
    fn test_no_notification_registered_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!(false, proto.get_notification_registered());
    }

    #[test]
    fn test_no_os_name_set() {
        let device: SDKDevice = serde_json::from_str("{}").unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!("", proto.get_platform());
    }

    #[test]
    fn test_os_name_ios() {
        let json = json!({
            "os_name": "iOS"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!("ios", proto.get_platform());
    }

    #[test]
    fn test_os_name_iphone_os() {
        let json = json!({
            "os_name": "iPhone OS"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!("ios", proto.get_platform());
    }

    #[test]
    fn test_os_name_android() {
        let json = json!({
            "os_name": "Android"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!("android", proto.get_platform());
    }

    #[test]
    fn test_platform_web() {
        let json = json!({
            "platform": "web"
        });

        let device: SDKDevice = serde_json::from_value(json).unwrap();
        let proto: proto_events::Device = device.into();

        assert_eq!("web", proto.get_platform());
    }
}
