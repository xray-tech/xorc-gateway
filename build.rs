use std::process::Command;

fn main() {
    Command::new("protoc")
        .args(&[
            "--rust_out",
            "src/proto_events",
            "--proto_path",
            "./third_party/events/schema/",
            "./third_party/events/schema/common/header.proto",
            "./third_party/events/schema/common/browser.proto",
            "./third_party/events/schema/common/device.proto",
            "./third_party/events/schema/common/carrier.proto",
            "./third_party/events/schema/sdk_event.proto",
        ])
        .status()
        .unwrap();
}
