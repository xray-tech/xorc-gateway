extern crate prost_build;

fn main() {
    prost_build::compile_protos(
        &[
            "third_party/events/common/header.proto",
            "third_party/events/common/browser.proto",
            "third_party/events/common/device.proto",
            "third_party/events/common/carrier.proto",
            "third_party/events/sdk_event.proto",
        ],
        &["third_party/events/"]
    ).unwrap();
}
