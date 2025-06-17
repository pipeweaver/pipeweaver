use semver::Version;

fn main() {
    let qt_version = std::env::var("DEP_QT_VERSION")
        .unwrap()
        .parse::<Version>()
        .expect("Parsing Qt version failed");

    // QTWebEngine isn't available before 6.2.0, so bail if that's not present
    if qt_version >= Version::new(6, 0, 0) && qt_version < Version::new(6, 2, 0) {
        panic!("QT Web Engine not available on this QT Version: {}", qt_version);
    }
}
