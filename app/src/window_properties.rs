use log::debug;
use qmetaobject::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct WindowGeometry {
    width: i32,
    height: i32,
    x: i32,
    y: i32,
}

#[derive(Default, QObject)]
pub struct WindowProperties {
    base: qt_base_class!(trait QObject),
    // Window geometry properties - each property needs a corresponding signal
    // for the NOTIFY mechanism, but the signals are handled automatically by Qt
    width: qt_property!(i32; NOTIFY width_changed),
    height: qt_property!(i32; NOTIFY height_changed),
    x: qt_property!(i32; NOTIFY x_changed),
    y: qt_property!(i32; NOTIFY y_changed),

    // Signal definitions required by qt_property! macros above
    width_changed: qt_signal!(),
    height_changed: qt_signal!(),
    x_changed: qt_signal!(),
    y_changed: qt_signal!(),

    // Custom signal for window closing
    close_requested: qt_signal!(),
    handle_close_request: qt_method!(fn(&mut self) -> bool),
}

impl WindowProperties {
    fn get_config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("pipeweaver");
        fs::create_dir_all(&path).ok();
        path.push("window.json");
        path
    }

    fn load_geometry() -> WindowGeometry {
        let path = Self::get_config_path();
        if let Ok(content) = fs::read_to_string(path)
            && let Ok(geometry) = serde_json::from_str::<WindowGeometry>(&content)
        {
            debug!(
                "Loaded geometry: {}x{} at ({}, {})",
                geometry.width, geometry.height, geometry.x, geometry.y
            );
            return geometry;
        }

        // Default values if file doesn't exist or is invalid
        let geometry = WindowGeometry {
            width: 1000, // Match minimumWidth from QML
            height: 600, // Match minimumHeight from QML
            x: 100,
            y: 100,
        };
        debug!(
            "Using default geometry: {}x{} at ({}, {})",
            geometry.width, geometry.height, geometry.x, geometry.y
        );
        geometry
    }

    pub fn new() -> Self {
        let geometry = Self::load_geometry();
        WindowProperties {
            width: geometry.width,
            height: geometry.height,
            x: geometry.x,
            y: geometry.y,

            ..Default::default()
        }
    }

    pub fn save_geometry(&self) {
        let geometry = WindowGeometry {
            width: self.width,
            height: self.height,
            x: self.x,
            y: self.y,
        };

        debug!(
            "Saving geometry: {}x{} at ({}, {})",
            geometry.width, geometry.height, geometry.x, geometry.y
        );

        if let Ok(json) = serde_json::to_string_pretty(&geometry) {
            let path = Self::get_config_path();
            fs::write(path, json).ok();
        }
    }

    pub fn handle_close_request(&mut self) -> bool {
        self.save_geometry();
        self.close_requested();
        true
    }
}
