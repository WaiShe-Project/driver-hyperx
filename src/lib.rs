//! WaiShe driver for HyperX wireless headsets.
//!
//! Protocol facts from HyperBatteryMonitor (MIT) `Devices/Cloud3WirelessAlternateReceiverDevice.cs`,
//! verified on hardware in M0. See `PROVENANCE.md` and `docs/cloud-iii-wireless.md`.
//!
//! Requests are output reports `66 <selector>`; answers arrive as input reports with the same
//! selector. The headset also reports changes on its own. While the headset is off the receiver
//! still answers, with zeros: only the battery answer can tell "off" from "on, not charging,
//! unmuted", so charging and mute readings are trusted only while the headset is known online.

use waishe_driver::hid::Handle;
use waishe_driver::{
    Access, CapabilityDescriptor, CapabilityEvent, Driver, EndpointInfo, Precision, ProbeError,
    ProbeResult, ReadError, Transport, Unavailable, Value, ValueKind, charging, id, link,
};

pub const REPORT_ID: u8 = 0x66;
const REPORT_LEN: usize = 62;
const QUERY_TIMEOUT_MS: u64 = 2000;
const CONTROL_USAGE_PAGE: u16 = 0xFF13;

/// Selectors of answers to our queries.
pub mod query {
    pub const BATTERY: u8 = 0x89;
    pub const CHARGING: u8 = 0x8A;
    pub const MIC_MUTE: u8 = 0x86;
}

/// Selectors of reports the headset sends on its own.
pub mod notify {
    pub const BATTERY: u8 = 0x0D;
    pub const CHARGING: u8 = 0x0C;
    pub const MIC_MUTE: u8 = 0x0A;
}

/// What one input report means, independent of transport. Pure, so it is unit-tested natively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// Battery answer or notification. `None` level: the headset is off.
    Battery(Option<u8>),
    Charging(&'static str),
    MicMuted(bool),
    Other,
}

pub fn parse(report: &[u8]) -> Report {
    if report.len() < 3 || report[0] != REPORT_ID {
        return Report::Other;
    }
    match report[1] {
        query::BATTERY | notify::BATTERY if report.len() >= 5 => {
            if report[2] == 0 && report[3] == 0 {
                Report::Battery(None)
            } else if report[4] <= 100 {
                Report::Battery(Some(report[4]))
            } else {
                Report::Other
            }
        }
        query::CHARGING | notify::CHARGING => Report::Charging(match report[2] {
            1 => charging::CHARGING,
            // Value 2 comes from HaloBattery's notes; not yet seen on hardware.
            2 => charging::FULL,
            _ => charging::DISCHARGING,
        }),
        query::MIC_MUTE | notify::MIC_MUTE => Report::MicMuted(report[2] == 1),
        _ => Report::Other,
    }
}

pub struct HyperX {
    control: Handle,
    /// `None` until the first battery answer.
    online: Option<bool>,
}

impl HyperX {
    /// Sends one query and waits for its answer, skipping unrelated reports.
    fn query(&self, selector: u8) -> Result<Report, ReadError> {
        let mut req = [0u8; REPORT_LEN];
        req[0] = REPORT_ID;
        req[1] = selector;
        self.control
            .write(&req)
            .map_err(|e| ReadError::Unavailable(e.into()))?;
        let deadline = waishe_driver::now_ms() + QUERY_TIMEOUT_MS;
        let mut buf = [0u8; REPORT_LEN];
        loop {
            let now = waishe_driver::now_ms();
            if now >= deadline {
                return Err(ReadError::Unavailable(Unavailable::Timeout));
            }
            match self.control.read(&mut buf, (deadline - now) as u32) {
                Err(e) => return Err(ReadError::Unavailable(e.into())),
                Ok(None) => return Err(ReadError::Unavailable(Unavailable::Timeout)),
                Ok(Some(n)) if n >= 2 && buf[0] == REPORT_ID && buf[1] == selector => {
                    return Ok(parse(&buf[..n]));
                }
                Ok(Some(_)) => {}
            }
        }
    }

    fn refresh_online(&mut self) -> Result<Option<u8>, ReadError> {
        match self.query(query::BATTERY)? {
            Report::Battery(level) => {
                self.online = Some(level.is_some());
                Ok(level)
            }
            _ => Err(ReadError::Protocol("unexpected battery answer".into())),
        }
    }

    fn require_online(&mut self) -> Result<(), ReadError> {
        match self.refresh_online()? {
            Some(_) => Ok(()),
            None => Err(ReadError::Unavailable(Unavailable::Disconnected)),
        }
    }
}

fn event(capability: &str, reading: Result<Value, ReadError>) -> CapabilityEvent {
    CapabilityEvent {
        capability: capability.into(),
        reading,
    }
}

fn link_value(online: bool) -> Value {
    Value::Enum(
        (if online {
            link::CONNECTED
        } else {
            link::DISCONNECTED
        })
        .into(),
    )
}

impl Driver for HyperX {
    fn probe(ep: &EndpointInfo) -> Result<(Self, ProbeResult), ProbeError> {
        let control = ep
            .interfaces
            .iter()
            .find(|i| i.usage_page == CONTROL_USAGE_PAGE)
            .map(|i| Handle(i.handle))
            .ok_or(ProbeError::NotMine)?;
        let model = match ep.product_id {
            0x0FCC => "cloud-iii-wireless",
            _ => return Err(ProbeError::NotMine),
        };
        // The receiver is the endpoint; the headset behind it may be off, which is a link state,
        // not a probe failure. Do not query here: probe must not depend on the headset.
        let driver = HyperX {
            control,
            online: None,
        };
        Ok((
            driver,
            ProbeResult {
                model: model.into(),
                stable_id: None,
                transport: Transport::Receiver,
                firmware: None,
                listen: vec![control.0],
            },
        ))
    }

    fn describe(&self) -> Vec<CapabilityDescriptor> {
        let states = |v: &[&str]| v.iter().map(|s| (*s).to_string()).collect();
        vec![
            CapabilityDescriptor::new(id::BATTERY_LEVEL, Access::READ_NOTIFY, ValueKind::Percent)
                .precision(Precision::Exact)
                .poll_every(300),
            CapabilityDescriptor::new(
                id::BATTERY_CHARGING,
                Access::READ_NOTIFY,
                ValueKind::Enum {
                    variants: states(&[charging::DISCHARGING, charging::CHARGING, charging::FULL]),
                },
            )
            .poll_every(300),
            CapabilityDescriptor::new(
                id::LINK_STATE,
                Access::READ_NOTIFY,
                ValueKind::Enum {
                    variants: states(&[link::CONNECTED, link::DISCONNECTED]),
                },
            )
            .poll_every(300),
            // No polling: the headset reports every change, and the host reads once when it starts.
            CapabilityDescriptor::new(id::HEADSET_MIC_MUTE, Access::READ_NOTIFY, ValueKind::Bool),
        ]
    }

    fn read(&mut self, capability: &str) -> Result<Value, ReadError> {
        match capability {
            id::LINK_STATE => Ok(link_value(self.refresh_online()?.is_some())),
            id::BATTERY_LEVEL => match self.refresh_online()? {
                Some(level) => Ok(Value::Percent(level)),
                None => Err(ReadError::Unavailable(Unavailable::Disconnected)),
            },
            id::BATTERY_CHARGING => {
                self.require_online()?;
                match self.query(query::CHARGING)? {
                    Report::Charging(s) => Ok(Value::Enum(s.into())),
                    _ => Err(ReadError::Protocol("unexpected charging answer".into())),
                }
            }
            id::HEADSET_MIC_MUTE => {
                self.require_online()?;
                match self.query(query::MIC_MUTE)? {
                    Report::MicMuted(m) => Ok(Value::Bool(m)),
                    _ => Err(ReadError::Protocol("unexpected mute answer".into())),
                }
            }
            _ => Err(ReadError::Unsupported),
        }
    }

    fn on_input(&mut self, _handle: u32, report: &[u8]) -> Vec<CapabilityEvent> {
        match parse(report) {
            Report::Battery(level) => {
                self.online = Some(level.is_some());
                vec![
                    event(id::LINK_STATE, Ok(link_value(level.is_some()))),
                    event(
                        id::BATTERY_LEVEL,
                        level
                            .map(Value::Percent)
                            .ok_or(ReadError::Unavailable(Unavailable::Disconnected)),
                    ),
                ]
            }
            // Zeros from an offline headset look like real values: drop them unless known online.
            Report::Charging(s) if self.online == Some(true) => {
                vec![event(id::BATTERY_CHARGING, Ok(Value::Enum(s.into())))]
            }
            Report::MicMuted(m) if self.online == Some(true) || report[1] == notify::MIC_MUTE => {
                // A mute-button report proves the headset is on.
                self.online = Some(true);
                vec![event(id::HEADSET_MIC_MUTE, Ok(Value::Bool(m)))]
            }
            _ => Vec::new(),
        }
    }
}

waishe_driver::export_driver!(HyperX);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_battery_is_not_a_level() {
        assert_eq!(parse(&[0x66, 0x89, 0, 0, 0]), Report::Battery(None));
        assert_eq!(
            parse(&[0x66, 0x89, 0x0e, 0xc9, 0x2f]),
            Report::Battery(Some(47))
        );
        assert_eq!(
            parse(&[0x66, 0x0d, 0x0e, 0xca, 0x2f]),
            Report::Battery(Some(47))
        );
    }

    #[test]
    fn mute_button_reports() {
        assert_eq!(parse(&[0x66, 0x0a, 0x01, 0x01]), Report::MicMuted(true));
        assert_eq!(parse(&[0x66, 0x0a, 0x00, 0x01]), Report::MicMuted(false));
    }

    #[test]
    fn unrelated_reports_are_ignored() {
        assert_eq!(parse(&[0x66, 0x07, 0x01]), Report::Other);
        assert_eq!(parse(&[0x01, 0x89, 0x0e, 0xc9, 0x2f]), Report::Other);
    }
}
