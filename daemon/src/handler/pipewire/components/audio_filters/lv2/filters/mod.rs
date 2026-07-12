use pipeweaver_shared::FilterValue;
use std::collections::HashMap;

pub(crate) mod generic;

fn defaults_map(uri: String) -> HashMap<String, FilterValue> {
    match uri.as_str() {
        "TEMP" => HashMap::new(),
        _ => HashMap::new(),
    }
}
