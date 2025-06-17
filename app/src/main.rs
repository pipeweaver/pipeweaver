mod app_state;

use qmetaobject::prelude::*;
use qmetaobject::webengine;

qrc!(pipeweaver_resources,
    "webengine" {
        "main.qml",
    },
);

fn main() {
    webengine::initialize();
    pipeweaver_resources();
    let mut engine = QmlEngine::new();
    engine.load_file("qrc:/webengine/main.qml".into());
    engine.exec();
}