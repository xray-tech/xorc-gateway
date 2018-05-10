pub mod common {
    include!(concat!(env!("OUT_DIR"), "/common.rs"));
}

pub mod events {
    include!(concat!(env!("OUT_DIR"), "/events.rs"));
}
