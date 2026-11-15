//! Linux: let the bridge find `libsteam_api.so` next to itself (`just build`
//! copies it there). macOS (`@loader_path`) and Windows (Godot adds the
//! extension's folder to the DLL search path) need nothing.

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_STEAM");
    let linux = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "linux");
    if linux && std::env::var_os("CARGO_FEATURE_STEAM").is_some() {
        println!("cargo:rustc-link-arg-cdylib=-Wl,-rpath,$ORIGIN");
    }
}
