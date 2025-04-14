use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Grab the current git revision of the project (useful for logging)
    let version = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "Unknown".to_string());

    println!("cargo:rustc-env=GIT_HASH={}", version);

    // Next up, we need to build the UI into the Daemon

    // npm install, to make sure the environment is set up
    Command::new("npm")
        .arg("install")
        .current_dir("../pipecast-web")
        .output()
        .expect("Unable to Run npm, cannot build");

    // Run the npm build
    Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir("../pipecast-web")
        .output()
        .expect("Unable to Run npm, cannot build");

    // Delete anything that exists already, and move the content in
    let content = Path::new("./web-content");
    if content.exists() {
        fs::remove_dir_all(content).expect("Error Deleting Directory!");
    }
    fs::rename("../pipecast-web/dist/", content).expect("Unable to install UI");
}