use crate::{APP_NAME, ICON};
use anyhow::Result;

use image::GenericImageView;

use crate::handler::packet::{Messenger, handle_packet};
use crate::stop::Stop;
use ksni::menu::StandardItem;
use ksni::{Category, Icon, MenuItem, Status, ToolTip, Tray, TrayMethods};
use log::debug;
use pipeweaver_ipc::commands::{DaemonCommand, DaemonRequest};
use std::sync::LazyLock;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

enum TrayMessages {
    Activate,
    Quit,
}

pub async fn spawn_tray(mut shutdown: Stop, sender: Messenger) -> Result<()> {
    debug!("Spawning Tray");

    let (icon_tx, mut icon_rx) = mpsc::channel(20);
    let icon = TrayIcon::new(icon_tx);
    let handle = icon.spawn_without_dbus_name().await?;

    loop {
        tokio::select! {
            Some(msg) = icon_rx.recv() => {
                match msg {
                    TrayMessages::Activate => {
                        debug!("Activate Triggered");
                        let message = DaemonRequest::Daemon(DaemonCommand::OpenInterface);
                        handle_packet(message, &sender).await?;
                    },
                    TrayMessages::Quit => {
                        debug!("Quit Triggered");
                        shutdown.trigger();
                        break;
                    }
                }
            },
            () = shutdown.recv() => {
                break;
            }
        }
    }

    debug!("Stopping Tray");
    if !handle.is_closed() {
        handle.shutdown();
    }

    // Remove the temporary icon file
    debug!("Tray Stopped");
    Ok(())
}

struct TrayIcon {
    tx: Sender<TrayMessages>,
}

impl TrayIcon {
    fn new(tx: Sender<TrayMessages>) -> Self {
        Self { tx }
    }
}

impl Tray for TrayIcon {
    fn id(&self) -> String {
        APP_NAME.to_string()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.try_send(TrayMessages::Activate);
    }
    fn category(&self) -> Category {
        Category::SystemServices
    }
    fn title(&self) -> String {
        APP_NAME.to_string()
    }
    fn status(&self) -> Status {
        Status::Active
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        static TRAY_ICON: LazyLock<Icon> = LazyLock::new(|| {
            let img = image::load_from_memory_with_format(ICON, image::ImageFormat::Png)
                .expect("Unable to Load Image");

            let (width, height) = img.dimensions();
            let mut data = img.into_rgba8().into_vec();

            for pixel in data.chunks_exact_mut(4) {
                pixel.rotate_right(1) // RGBA to ARGB
            }

            Icon {
                width: width as i32,
                height: height as i32,
                data,
            }
        });

        vec![TRAY_ICON.clone()]
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: String::from(APP_NAME),
            description: String::from("PipeWeaver Audio Mixer"),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: String::from("Show"),
                activate: Box::new(|this: &mut TrayIcon| {
                    let _ = this.tx.try_send(TrayMessages::Activate);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: String::from("Quit"),
                activate: Box::new(|this: &mut TrayIcon| {
                    let _ = this.tx.try_send(TrayMessages::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
