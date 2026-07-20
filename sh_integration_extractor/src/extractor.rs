use std::os::raw::{c_char, c_int, c_void};

use sh_integration_sys::scanner::{get_scanner_manager, ScannerEventType};

use crate::index::index_file;
use crate::remove::remove_file;

#[repr(C)]
pub struct ScannerEvent {
    pub event_type: c_int,
    pub path: *mut c_char,
    pub lipchandle: *mut c_void,
    pub filename: *mut c_char,
    pub uuid: *mut c_char,
    pub glob: *mut c_char,
}

pub type ScannerEventHandler = extern "C" fn(event: *const ScannerEvent) -> c_int;

extern "C" fn extractor_callback(event: *const ScannerEvent) -> c_int {
    if event.is_null() {
        return 1;
    }
    let event = unsafe { &*event };

    let event_type = match ScannerEventType::from_i32(event.event_type) {
        Some(t) => t,
        None => {
            eprintln!("Received unknown event: {}", event.event_type);
            return 1;
        }
    };

    let path = unsafe { std::ffi::CStr::from_ptr(event.path).to_string_lossy() };
    let filename = if event.filename.is_null() {
        String::new()
    } else {
        unsafe {
            std::ffi::CStr::from_ptr(event.filename)
                .to_string_lossy()
                .into_owned()
        }
    };
    let uuid = if event.uuid.is_null() {
        String::new()
    } else {
        unsafe {
            std::ffi::CStr::from_ptr(event.uuid)
                .to_string_lossy()
                .into_owned()
        }
    };

    eprintln!(
        "sh_integration extractor called with event type {:?}",
        event_type
    );
    eprintln!(
        "event_type={:?} filename={} path={} uuid={}",
        event_type, filename, path, uuid
    );

    let scanner = get_scanner_manager();

    match event_type {
        ScannerEventType::Add => {
            index_file(&path, &filename, true, scanner);
            0
        }
        ScannerEventType::Delete => {
            remove_file(&path, &filename, Some(&uuid), scanner);
            0
        }
        ScannerEventType::Update => {
            remove_file(&path, &filename, Some(&uuid), scanner);
            index_file(&path, &filename, false, scanner);
            0
        }
        ScannerEventType::AddThumb | ScannerEventType::UpdateThumb => 0,
    }
}

#[no_mangle]
pub extern "C" fn load_extractor(handler: *mut *mut ScannerEventHandler, unk1: *mut c_int) -> c_int {
    eprintln!("sh_integration extractor v4.1.0 initialised");
    unsafe {
        *handler = extractor_callback as *mut ScannerEventHandler;
        *unk1 = 0;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_conversion() {
        assert_eq!(ScannerEventType::from_i32(0), Some(ScannerEventType::Add));
        assert_eq!(
            ScannerEventType::from_i32(1),
            Some(ScannerEventType::Delete)
        );
        assert_eq!(
            ScannerEventType::from_i32(2),
            Some(ScannerEventType::Update)
        );
        assert_eq!(
            ScannerEventType::from_i32(3),
            Some(ScannerEventType::AddThumb)
        );
        assert_eq!(
            ScannerEventType::from_i32(4),
            Some(ScannerEventType::UpdateThumb)
        );
    }

    #[test]
    fn test_invalid_event_type() {
        assert_eq!(ScannerEventType::from_i32(5), None);
        assert_eq!(ScannerEventType::from_i32(99), None);
        assert_eq!(ScannerEventType::from_i32(-1), None);
    }
}
