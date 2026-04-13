// Ok, this is somewhat of a hack, but if we're running under KDE we can use dbus scripting
// to force our window to the front when it's activated. This allows us to respond correctly when
// being triggered via the Pipeweaver tray icon.
// Wayland sucks :D
//
// Thanks to DesignGears for the idea :)

use crate::APP_NAME;
use std::env::temp_dir;
use std::error::Error;
use std::{env, fs, process};
use zbus::blocking::Connection;
use zbus::proxy;

#[proxy(
    interface = "org.kde.kwin.Scripting",
    default_service = "org.kde.KWin",
    default_path = "/Scripting",
    gen_blocking = true
)]
trait KWinScripting {
    #[zbus(name = "loadScript")]
    fn load_script(&self, file_path: &str, plugin_name: &str) -> zbus::Result<i32>;

    #[zbus(name = "unloadScript")]
    fn unload_script(&self, plugin_name: &str) -> zbus::Result<bool>;
}

#[proxy(
    interface = "org.kde.kwin.Script",
    default_service = "org.kde.KWin",
    default_path = "/Scripting/Script0", // overridden at runtime
    gen_blocking = true
)]
trait KWinScript {
    #[zbus(name = "run")]
    fn run(&self) -> zbus::Result<()>;
}

pub fn raise_window() -> Result<(), Box<dyn Error>> {
    let conn = Connection::session()?;
    let pid = process::id();

    let condition = if env::var("FLATPAK_SANDBOX_DIR").is_ok() {
        format!("w[i].desktopFileName == {}", APP_NAME)
    } else {
        format!("w[i].pid === {pid}")
    };

    // This script loops through all the active windows, looks for the one assigned to our
    // pid, then flags it active in the workspace.
    let script = format!(
        "var w = workspace.windowList(); \
         for (var i in w) {{ \
             if ({condition}) {{ workspace.activeWindow = w[i]; break; }} \
         }}"
    );

    let scripting = KWinScriptingProxyBlocking::new(&conn)?;
    let plugin = format!("kwin_raise_{pid}");

    let tmp_path = temp_dir().join("{plugin}.js");
    fs::write(&tmp_path, &script)?;

    // Wrap everything in a result, so we can ensure the temp file is deleted before returning
    let result = {
        let file_path = tmp_path.to_str().ok_or("non-UTF8 temp path")?;
        let script_id = scripting.load_script(file_path, &plugin)?;

        KWinScriptProxyBlocking::builder(&conn)
            .path(format!("/Scripting/Script{script_id}"))?
            .build()?
            .run()?;

        scripting.unload_script(&plugin)?;
        Ok::<_, Box<dyn Error>>(())
    };

    let _ = fs::remove_file(&tmp_path);
    result?;

    Ok(())
}
