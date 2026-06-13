//! Env-gated TypeScript file writer.
//!
//! Pattern borrowed from `by-macros/src/write_file.rs`, with one critical
//! change: it reads `std::env::var("TS_SERVER_FN_PACKAGE_DIR")` **at macro
//! expansion time** rather than `option_env!`. `option_env!` bakes the value
//! in when the *macro crate itself* compiles, so toggling it would require
//! rebuilding `ts-server-fn`. `std::env::var` reads the *consumer* build's
//! environment, so:
//!
//!   TS_SERVER_FN_PACKAGE_DIR=… cargo check  →  files written
//!   (unset)                                  →  nothing written
//!
//! …without ever rebuilding this crate.

use std::fs;
use std::path::Path;

/// Returns the configured package dir, or `None` when generation is off.
pub fn package_dir() -> Option<String> {
    match std::env::var("TS_SERVER_FN_PACKAGE_DIR") {
        Ok(d) if !d.trim().is_empty() => Some(d),
        _ => None,
    }
}

/// Write one handler's TS source under `$DIR/src/handlers/<feature>/<fn>.ts`.
///
/// `feature` is a directory segment used to group handlers (the consumer
/// passes the handler module path's leaf, or "" for a flat layout). The
/// file stem is the camelCase fn name. Writes are best-effort: a failure is
/// not allowed to break the consumer's compile, so errors are swallowed
/// (the macro's primary job — re-emitting the server fn — must always
/// succeed).
pub fn write_handler(dir: &str, feature: &str, fn_name_camel: &str, source: &str) {
    let mut sub = format!("{dir}/src/handlers");
    if !feature.is_empty() {
        sub.push('/');
        sub.push_str(feature);
    }

    if let Err(_e) = fs::create_dir_all(&sub) {
        return;
    }

    let file_path = format!("{sub}/{fn_name_camel}.ts");
    let _ = fs::write(Path::new(&file_path), source);
}
