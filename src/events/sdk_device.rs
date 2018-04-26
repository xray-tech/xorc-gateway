use proto_events;

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
    pub fn platform(&self) -> Option<&'static str>
    {
        self.os_name.as_ref().map(|n| {
            match n.as_ref() {
                "iOS" | "iPhone OS" => "ios",
                "Android" => "android",
                _ => ""
            }
        })
    }
}

impl Into<proto_events::Device> for SDKDevice {
    fn into(self) -> proto_events::Device {
        let mut device = proto_events::Device::new();

        if let Some(ref platform) = self.platform() {
            device.set_platform(platform.to_string());
        }

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
