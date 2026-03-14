// Copyright 2025 Signal Messenger, LLC
// SPDX-License-Identifier: AGPL-3.0-only

fn main() {
    let protos = ["src/proto/pq_ratchet.proto"];
    let mut prost_build = prost_build::Config::new();
    prost_build
        .compile_protos(&protos, &["src"])
        .expect("Protobufs in src are valid");

    // Gate as_str_name/from_str_name with #[cfg(not(feature = "extraction"))] — these
    // return &'static str which Aeneas cannot translate ("no bottoms in value" error),
    // and they are never called by spqr.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let path = format!("{out_dir}/signal.proto.pq_ratchet.rs");
    let content = std::fs::read_to_string(&path).unwrap();
    let content = content
        .replace("pub fn as_str_name(", "#[cfg(not(feature = \"extraction\"))]\n        pub fn as_str_name(")
        .replace("pub fn from_str_name(", "#[cfg(not(feature = \"extraction\"))]\n        pub fn from_str_name(");
    std::fs::write(&path, content).unwrap();

    for proto in &protos {
        println!("cargo:rerun-if-changed={proto}");
    }
}
