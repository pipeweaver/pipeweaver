import QtQuick 2.12
import QtQuick.Controls 2.12
import QtWebEngine 1.8

ApplicationWindow {
    id: mainWindow
    visible: true
    width: 800
    height: 600

    // It should be noted, that this doesn't trigger under Wayland on Minimise, there's not
    // really anything we can do about that.
    onWindowStateChanged: function(windowState) {
        if (webView.loading) {
            console.log("WebView still loading, skipping visibility update")
            return
        }
        const isHidden = (windowState & Qt.WindowMinimized) === Qt.WindowMinimized
        const updateScript = `window._setVisibility(${isHidden});`
        webView.runJavaScript(updateScript, function(result) {
            console.log("Visibility update completed")
        })
    }

    WebEngineView {
        id: webView
        anchors.fill: parent
        url: "http://localhost:14565"

        onLoadingChanged: function(loadRequest) {
            if (loadRequest.status === WebEngineView.LoadSucceededStatus) {
                const initScript = `
                    (function() {
                        if (!window._visibilityInitialized) {
                            let _hidden = false;
                            let _visibilityState = 'visible';

                            Object.defineProperty(document, 'hidden', {
                                get: function() { return _hidden; }
                            });

                            Object.defineProperty(document, 'visibilityState', {
                                get: function() { return _visibilityState; }
                            });

                            window._setVisibility = function(hidden) {
                                _hidden = hidden;
                                _visibilityState = hidden ? 'hidden' : 'visible';
                                document.dispatchEvent(new Event('visibilitychange'));
                            };

                            window._visibilityInitialized = true;
                        }
                    })();
                `
                webView.runJavaScript(initScript, function(result) {
                    console.log("Visibility properties initialized")
                })
            }
        }
    }
}