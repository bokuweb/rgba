// Draws the art and sound with the C version's generator and links with its
// startup code's linker script, so both builds share one set of assets.
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let c_dir = manifest.join("../flappy-enaga");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let generator = c_dir.join("tools/gen_assets.py");
    let status = Command::new("python3")
        .arg(&generator)
        .arg(&out)
        .arg("--rust")
        .stdout(std::process::Stdio::null())
        .status()
        .expect("python3 is needed to generate the assets");
    assert!(status.success(), "gen_assets.py failed");

    let script = c_dir.join("rt/gba.ld");
    println!("cargo:rustc-link-arg=-T{}", script.display());
    println!("cargo:rerun-if-changed={}", generator.display());
    println!("cargo:rerun-if-changed={}", script.display());
    println!("cargo:rerun-if-changed={}", c_dir.join("rt/crt0.s").display());
}
