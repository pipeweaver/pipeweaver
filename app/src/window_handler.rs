use log::{debug, warn};
use qmetaobject::prelude::*;
use std::sync::mpsc;

pub enum WindowMessage {
    Trigger,
    Close,
    /// Health check — Qt should send back () on the provided sender immediately.
    Ping(mpsc::SyncSender<()>),
}

#[derive(QObject)]
pub struct WindowHandler {
    rx: mpsc::Receiver<WindowMessage>,
    base: qt_base_class!(trait QObject),

    // Called to focus the QT Window
    trigger: qt_signal!(),
    on_trigger: qt_method!(
        fn on_trigger(&self) {
            self.trigger();
        }
    ),

    // Called to close the QT Window
    close: qt_signal!(),
    on_close: qt_method!(
        fn on_close(&self) {
            self.close();
        }
    ),

    // Allow QT to open a browser if a _blank link is clicked
    open_url: qt_method!(
        fn open_url(&self, url: String) {
            if let Err(e) = open::that_detached(url) {
                warn!("Failed to open URL in browser: {e}");
            }
        }
    ),

    // Called from QT to probe the message queue
    check_notifications: qt_method!(
        fn check_notifications(&self) {
            while let Ok(msg) = self.rx.try_recv() {
                match msg {
                    WindowMessage::Trigger => {
                        self.on_trigger();
                    }
                    WindowMessage::Close => {
                        self.on_close();
                    }
                    WindowMessage::Ping(tx) => {
                        debug!("Qt is alive! Responding to PING");
                        let _ = tx.send(());
                    }
                }
            }
        }
    ),
}

impl WindowHandler {
    pub fn new(rx: mpsc::Receiver<WindowMessage>) -> Self {
        Self {
            rx,
            base: Default::default(),

            trigger: Default::default(),
            on_trigger: Default::default(),

            close: Default::default(),
            on_close: Default::default(),

            open_url: Default::default(),

            check_notifications: Default::default(),
        }
    }
}
