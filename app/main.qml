import QtQuick
import QtQuick.Controls
import QtWebEngine

ApplicationWindow {
    id: mainWindow
    title: "Pipeweaver"

    minimumWidth: 1000
    minimumHeight: 600

    // These all come from Rust via the WindowProperties QObject
    width: windowProperties ? windowProperties.width : 1000
    height: windowProperties ? windowProperties.height : 600
    x: windowProperties ? windowProperties.x : 100
    y: windowProperties ? windowProperties.y : 100
    visible: true

    // This is the IPC handler from rust
    Connections {
        target: windowHandler
        enabled: windowHandler != null

        // Raise and Activate the window (on X11 this will bring to front, on Wayland it will highlight)
        function onTrigger() {
            mainWindow.raise()
            mainWindow.requestActivate()
        }

        // Close the Window
        function onClose() {
            mainWindow.close()
        }
    }


    Timer {
        id: windowHandlerPollTimer
        interval: 20
        repeat: true
        running: true
        onTriggered: {
            // Make sure the IPC Handler is connected
            if (!windowHandler) {
                return
            }

            // Ask rust to check for message notifications
            windowHandler.check_notifications()
        }
    }

    // Close request from Rust
    Connections {
        target: windowProperties
        enabled: true
    }

    // This is the geometry manager back to rust, so state can be stored
    Timer {
        id: geometryChangeTimer
        interval: 250
        repeat: false
        onTriggered: {
            if (windowProperties && mainWindow.visible) {
                windowProperties.width = mainWindow.width
                windowProperties.height = mainWindow.height
                windowProperties.x = mainWindow.x
                windowProperties.y = mainWindow.y
            }
        }
    }

    // Every 10 seconds we want to force a garbage collection, to hopefully keep the memory usage down.
    Timer {
        id: forceJavaScriptGC
        interval: 10000
        repeat: true
        running: true
        onTriggered: {
            webView.runJavaScript(`
                if (window.gc) {
                    window.gc();
                    setTimeout(() => window.gc(), 50);
                }
            `);
        }
    }

    // When the Window is closed, throw back to the windowProperties to handle any final saving
    onClosing: {
        if (windowProperties) {
            windowProperties.handle_close_request()
        }
    }
    onWidthChanged: geometryChangeTimer.restart()
    onHeightChanged: geometryChangeTimer.restart()
    onXChanged: geometryChangeTimer.restart()
    onYChanged: geometryChangeTimer.restart()

    WebEngineView {
        id: webView
        anchors.fill: parent
        property string initialUrl: "http://localhost:14565"

        Component.onCompleted: {
            url = initialUrl
        }

        onNewWindowRequested: function(request) {
            // Ignore the in-app navigation and hand off to the default browser
            request.action = WebEngineNewWindowRequest.IgnoreRequest
            windowHandler.open_url(request.requestedUrl.toString())
        }

        settings.pluginsEnabled: false
    }
}