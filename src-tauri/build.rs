fn main() {
    println!(
        "cargo:rustc-env=IMAPSYNC_TARGET={}",
        std::env::var("TARGET").unwrap()
    );
    tauri_build::build()
}
