extern crate prost_build;

fn main() {
    prost_build::compile_protos(
        &[
            "third_party/events/schema/common/header.proto",
            "third_party/events/schema/common/browser.proto",
            "third_party/events/schema/common/device.proto",
            "third_party/events/schema/common/carrier.proto",
            "third_party/events/schema/sdk_event.proto",
        ],
        &["third_party/events/schema/"]
    ).unwrap();
}
