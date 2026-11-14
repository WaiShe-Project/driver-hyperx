//! Runs the built driver (`target/wasm32-unknown-unknown/release/driver_hyperx.wasm`) against
//! exchanges recorded in M0 (`captures/cloud-iii-wireless-0fcc-2026-11-14.md`).
//! Build first: `cargo build --release --target wasm32-unknown-unknown`.

use waishe_driver_replay::types::cap::id;
use waishe_driver_replay::types::{EndpointInfo, HidInterface, ReadError, Unavailable, Value};
use waishe_driver_replay::{Replay, instantiate};

const WASM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/target/wasm32-unknown-unknown/release/driver_hyperx.wasm"
);

fn endpoint() -> EndpointInfo {
    EndpointInfo {
        vendor_id: 0x03F0,
        product_id: 0x0FCC,
        product: "HyperX Cloud III Wireless".into(),
        interfaces: vec![HidInterface {
            handle: 0,
            interface_number: 3,
            usage_page: 0xFF13,
            usage: 0x0001,
        }],
    }
}

fn run(script: &str) -> waishe_driver_replay::DriverInstance<Replay> {
    let mut d = instantiate(WASM, Replay::parse(script));
    let probe = d.probe(&endpoint()).unwrap().unwrap();
    assert_eq!(probe.model, "cloud-iii-wireless");
    assert_eq!(probe.listen, vec![0]);
    d
}

#[test]
fn reads_battery_while_online() {
    let mut d = run("
        write 0: 66 89 ..
        read  0: 66 89 0e c9 2f ..
    ");
    assert_eq!(d.read(id::BATTERY_LEVEL).unwrap(), Ok(Value::Percent(47)));
    d.backend().assert_finished();
}

#[test]
fn offline_headset_is_unavailable_not_zero() {
    // Captured with the headset switched off: the receiver answers with zeros.
    let mut d = run("
        write 0: 66 89 ..
        read  0: 66 89 00 00 00 ..
        write 0: 66 89 ..
        read  0: 66 89 00 00 00 ..
        write 0: 66 89 ..
        read  0: 66 89 00 00 00 ..
    ");
    let off = Err(ReadError::Unavailable(Unavailable::Disconnected));
    assert_eq!(d.read(id::BATTERY_LEVEL).unwrap(), off);
    // Charging and mute are gated on the battery answer and never queried while offline.
    assert_eq!(d.read(id::BATTERY_CHARGING).unwrap(), off);
    assert_eq!(d.read(id::HEADSET_MIC_MUTE).unwrap(), off);
    d.backend().assert_finished();
}

#[test]
fn skips_unrelated_reports_while_waiting() {
    // A mute-button report arrives between the query and its answer.
    let mut d = run("
        write 0: 66 89 ..
        read  0: 66 89 0e c9 2f ..
        write 0: 66 86 ..
        read  0: 66 0a 01 01 ..
        read  0: 66 86 01 ..
    ");
    assert_eq!(d.read(id::HEADSET_MIC_MUTE).unwrap(), Ok(Value::Bool(true)));
    d.backend().assert_finished();
}

#[test]
fn no_answer_is_a_timeout() {
    let mut d = run("
        write 0: 66 89 ..
        read  0: timeout
    ");
    assert_eq!(
        d.read(id::BATTERY_LEVEL).unwrap(),
        Err(ReadError::Unavailable(Unavailable::Timeout))
    );
}

#[test]
fn unsolicited_reports_become_events() {
    let mut d = run("");
    // Charging zeros before the headset is known online are dropped.
    assert!(d.on_input(0, &[0x66, 0x8a, 0x00]).unwrap().is_empty());
    let ev = d.on_input(0, &[0x66, 0x0a, 0x01, 0x01]).unwrap();
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0].capability, id::HEADSET_MIC_MUTE);
    assert_eq!(ev[0].reading, Ok(Value::Bool(true)));
    // Power-on burst from the capture.
    let ev = d.on_input(0, &[0x66, 0x0d, 0x0e, 0xca, 0x2f]).unwrap();
    assert!(
        ev.iter()
            .any(|e| e.capability == id::BATTERY_LEVEL && e.reading == Ok(Value::Percent(47)))
    );
    assert!(ev.iter().any(
        |e| e.capability == id::LINK_STATE && e.reading == Ok(Value::Enum("connected".into()))
    ));
    assert!(d.on_input(0, &[0x66, 0x07, 0x01]).unwrap().is_empty());
}
