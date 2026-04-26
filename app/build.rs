use cpp_build::Config;
use semver::Version;

fn main() {
    // Force rebuild whenever we change
    println!("cargo:rerun-if-changed=src/main.rs");

    let qt_version = std::env::var("DEP_QT_VERSION")
        .unwrap()
        .parse::<Version>()
        .expect("Parsing Qt version failed");

    // QTWebEngine isn't available before 6.2.0, so bail if that's not present
    if qt_version < Version::new(6, 2, 0) {
        panic!(
            "QT Web Engine not available on this QT Version: {}",
            qt_version
        );
    }

    let mut cfg = Config::new();
    cfg.flag_if_supported("-std=c++17");

    // Try pkg-config first (recommended on system installs)
    if let Ok(lib) = pkg_config::Config::new().probe("Qt6Gui") {
        for include_path in lib.include_paths {
            cfg.include(include_path);
        }
    } else {
        panic!("Unable to find Qt6 installation via pkg-config");
    }

    if let Ok(lib) = pkg_config::Config::new().probe("Qt6QuickControls2") {
        for include_path in lib.include_paths {
            cfg.include(include_path);
        }
    } else {
        panic!("Unable to find Qt6QuickControls2 via pkg-config");
    }

    cfg.build("src/main.rs");
}
