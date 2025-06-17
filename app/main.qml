import QtQuick 2.6;
import QtQuick.Window 2.0;
import QtWebEngine 1.4

Window {
    visible: true
    title: "PipeWeaver"
    width: 1024
    height: 800

    WebEngineView {
        anchors.fill: parent
        url: "http://localhost:14565"
    }
}