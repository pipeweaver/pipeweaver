use serde::{Deserialize, Serialize};

/// Ok, this time around I'm going to use Serde's 'default' feature, rather than having to
/// have everything as an Option<T> and fixing it later
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    #[serde(default = "default_profile")]
    profile: String,
}

fn default_profile() -> String {
    String::from("default")
}
