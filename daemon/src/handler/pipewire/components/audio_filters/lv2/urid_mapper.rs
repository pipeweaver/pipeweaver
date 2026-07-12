use std::collections::HashMap;
use std::ffi::{CStr, c_void};
use std::sync::Mutex;

pub struct UridMapper {
    map: Mutex<HashMap<String, u32>>,
    next_id: Mutex<u32>,
}

impl UridMapper {
    pub(crate) fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }

    fn map_uri(&self, uri: *const i8) -> u32 {
        if uri.is_null() {
            return 0;
        }

        let uri = unsafe { CStr::from_ptr(uri).to_string_lossy().into_owned() };

        let mut map = self.map.lock().unwrap();

        if let Some(id) = map.get(&uri) {
            return *id;
        }

        let mut next = self.next_id.lock().unwrap();
        let id = *next;
        *next += 1;

        map.insert(uri, id);
        id
    }
}

pub unsafe extern "C" fn urid_map_callback(handle: *mut c_void, uri: *const i8) -> u32 {
    let mapper = unsafe { &*(handle as *const UridMapper) };
    mapper.map_uri(uri)
}
