use events::output::common;
use std::net::IpAddr;
use blake2::{Blake2b, Digest};
use base64;
use maxminddb::geoip2::Country;
use ::GEOIP;

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
    #[serde(default)] pub ifa_tracking_enabled: bool,
    #[serde(default)] pub notification_registered: bool,
    pub h: Option<i32>,
    pub w: Option<i32>,
    pub locale: Option<String>,
    pub language: Option<String>,
    pub time_zone: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub os_version: Option<String>,
    pub os_name: Option<String>,
    pub network_connection_type: Option<String>,
    pub device_name: Option<String>,
    pub ifa: Option<String>,
    pub idfv: Option<String>,
    pub carrier_name: Option<String>,
    pub carrier_country: Option<String>,
    pub browser_name: Option<String>,
    pub browser_version: Option<String>,
    pub browser_ua: Option<String>,
    pub notification_types: Option<i32>,
    pub orientation: Option<String>,
    pub platform: Option<String>,
    pub ip_hashed_blake2: Option<String>,
    pub country: Option<String>,
}

impl SDKDevice {
    pub fn set_location(&mut self, ip: &IpAddr) {
        let data = format!("{}", ip);

        let mut hasher = Blake2b::new();
        hasher.input(data.as_bytes());

        let hash = hasher.result();
        self.ip_hashed_blake2 = Some(base64::encode(hash.as_slice()));

        self.country = GEOIP.lookup(ip.clone())
            .ok()
            .and_then(|res: Country| res.country)
            .and_then(|res| res.iso_code);
    }
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
            ip_hashed_blake2: self.ip_hashed_blake2,
            country: self.country,

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

    #[test]
    fn test_set_real_location_ipv4() {
        let json = json!({
            "platform": "web"
        });

        let ip_addr: IpAddr = "109.68.226.154".parse().unwrap();

        let mut device: SDKDevice = serde_json::from_value(json).unwrap();
        device.set_location(&ip_addr);

        let proto: common::Device = device.into();

        assert_eq!(
            Some(String::from("KOx83wHFu0JCTyitEEfU0+J0GWN5OxtXgMeIzDUinonr8ya0IY5VyYtrbDu8tRlBSo/a1T70lQ3uYcnSRYiR8w==")),
            proto.ip_hashed_blake2,
        );

        assert_eq!(
            Some(String::from("DE")),
            proto.country
        );
    }

    #[test]
    fn test_set_real_location_ipv6() {
        let json = json!({
            "platform": "web"
        });

        let ip_addr: IpAddr = "2001:4860:4860::8844".parse().unwrap();

        let mut device: SDKDevice = serde_json::from_value(json).unwrap();
        device.set_location(&ip_addr);

        let proto: common::Device = device.into();

        assert_eq!(
            Some(String::from("WlYX3LhjGXz3WRjP6V9dzs33shbfQhI+uFug7ft+W7VzzEgQPr6aMQxKn4yI7FF6MFiii+3OI2O0i9niN2Zt+g==")),
            proto.ip_hashed_blake2,
        );

        assert_eq!(
            Some(String::from("US")),
            proto.country
        );
    }

    #[test]
    fn test_set_local_ipv4() {
        let json = json!({
            "platform": "web"
        });

        let ip_addr: IpAddr = "127.0.0.1".parse().unwrap();

        let mut device: SDKDevice = serde_json::from_value(json).unwrap();
        device.set_location(&ip_addr);

        let proto: common::Device = device.into();

        assert_eq!(
            Some(String::from("+s8vULlRmH7/43pUpb2l4/B+olwJ9HPFLWgvsG+FJPw2qdR+UzgO1BuBGBMIQ61PjS6yHj6En9SaLUB1M6rXog==")),
            proto.ip_hashed_blake2,
        );

        assert_eq!(
            None,
            proto.country
        );
    }

    #[test]
    fn test_set_local_ipv6() {
        let json = json!({
            "platform": "web"
        });

        let ip_addr: IpAddr = "::1".parse().unwrap();

        let mut device: SDKDevice = serde_json::from_value(json).unwrap();
        device.set_location(&ip_addr);

        let proto: common::Device = device.into();

        assert_eq!(
            Some(String::from("iJYy4kyl+U+iXvriFVHa8S+fPnIwlSFwd6BiESUmVwpQA2/EiUZCwGRPtETJj2d89wr9svyy7S4E0Lb9mnER6w==")),
            proto.ip_hashed_blake2,
        );

        assert_eq!(
            None,
            proto.country
        );
    }
}
