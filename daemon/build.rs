use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../web");

    // Grab the current git revision of the project (useful for logging)
    let version = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "Unknown".to_string());

    println!("cargo:rustc-env=GIT_HASH={}", version);

    println!("Building Web Interface..");

    // Next up, we need to build the UI into the Daemon
    Command::new("npm")
        .arg("install")
        .current_dir("../web")
        .output()
        .expect("Unable to Run npm, cannot build");

    // Run the npm build
    Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir("../web")
        .output()
        .expect("Unable to Run npm, cannot build");

    // Delete anything that exists already, and move the content in
    let content = Path::new("./web-content");
    if content.exists() {
        fs::remove_dir_all(content).expect("Error Deleting Directory!");
    }
    fs::rename("../web/dist/", content).expect("Unable to install UI");
}
