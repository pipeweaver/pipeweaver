use anyhow::{bail, Context, Result};
use log::{debug, warn};
use pipecast_profile::Profile;
use std::fs::File;
use std::path::Path;

pub struct ProfileAdaptor {
    name: String,
    profile: Profile,
}

impl ProfileAdaptor {
    pub fn from_named(name: String, path: &Path) -> Profile {
        let path = path.join(format!("{}.pipecast", name));

        if path.is_file() {
            debug!("Loading profile from {}", path.to_string_lossy());
            let file = File::open(path);
            if let Ok(file) = file {
                if let Ok(profile) = serde_json::from_reader(file) {
                    return profile;
                }
            }

            warn!("Unable to load profile, returning a default");
            return Profile::default();
        }
        Profile::default()
    }
}