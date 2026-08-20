use std::os::raw::{c_char, c_int, c_void};

use kompanion_core::change_request::generate_change_request;

use kompanion_sys::scanner::{get_scanner_manager, ScannerEventType};

use crate::indexer::find_indexer;

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
            log::warn!("Received unknown event: {}", event.event_type);
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

    log::info!(
        "kompanion extractor called with event type {:?}",
        event_type
    );
    log::info!(
        "event_type={:?} filename={} path={} uuid={}",
        event_type,
        filename,
        path,
        uuid
    );

    let scanner = get_scanner_manager();

    let indexer = match find_indexer(&filename) {
        Some(i) => i,
        None => {
            log::debug!("No indexer found for: {}", filename);
            return 0;
        }
    };

    match event_type {
        ScannerEventType::Add => {
            let full_path = format!("{}/{}", path, filename);

            let mut uuid_buf = [0u8; 37];
            scanner.gen_uuid(&mut uuid_buf);
            let uuid_str =
                String::from_utf8(uuid_buf.iter().take_while(|&&b| b != 0).copied().collect())
                    .unwrap_or_default();

            let metadata = match indexer.extract_metadata(&full_path) {
                Ok(m) => m,
                Err(e) => {
                    log::error!("Failed to extract metadata: {}", e);
                    return 1;
                }
            };

            let icon = match indexer.handle_sdr(&full_path, &metadata) {
                Ok(i) => i,
                Err(e) => {
                    log::error!("Failed to handle SDR: {}", e);
                    return 1;
                }
            };

            if indexer.supports_hooks() {
                indexer.on_install(&full_path);
            }

            let json = generate_change_request(
                &full_path,
                &uuid_str,
                metadata.name.as_deref(),
                metadata.author.as_deref(),
                icon.as_deref(),
                true,
                indexer.mime_type(),
                indexer.cde_type(),
                if metadata.extra.is_empty() {
                    None
                } else {
                    Some(&metadata.extra)
                },
            );

            let result = scanner.post_change(&json);
            let json_str = serde_json::to_string_pretty(&json).unwrap_or_default();
            log::debug!("Indexing json:\n{}", json_str);
            log::debug!("ccat error: {}", result);
            0
        }
        ScannerEventType::Delete => {
            let full_path = format!("{}/{}", path, filename);

            if !uuid.is_empty() {
                log::info!("Removing ccat entry.");
                scanner.delete_ccat_entry(&uuid);
            }

            if indexer.supports_hooks() {
                indexer.on_remove(&full_path);
            }

            let sdr_path = format!("{}.sdr", full_path);
            if std::path::Path::new(&sdr_path).exists() {
                log::info!("SDR exists - deleting");
                let _ = std::fs::remove_dir_all(&sdr_path);
            }
            0
        }
        ScannerEventType::Update => {
            let full_path = format!("{}/{}", path, filename);

            if !uuid.is_empty() {
                scanner.delete_ccat_entry(&uuid);
            }
            if indexer.supports_hooks() {
                indexer.on_remove(&full_path);
            }
            let sdr_path = format!("{}.sdr", full_path);
            let _ = std::fs::remove_dir_all(&sdr_path);

            let mut new_uuid_buf = [0u8; 37];
            scanner.gen_uuid(&mut new_uuid_buf);
            let new_uuid = String::from_utf8(
                new_uuid_buf
                    .iter()
                    .take_while(|&&b| b != 0)
                    .copied()
                    .collect(),
            )
            .unwrap_or_default();

            let metadata = match indexer.extract_metadata(&full_path) {
                Ok(m) => m,
                Err(e) => {
                    log::error!("Failed to extract metadata: {}", e);
                    return 1;
                }
            };

            let icon = match indexer.handle_sdr(&full_path, &metadata) {
                Ok(i) => i,
                Err(e) => {
                    log::error!("Failed to handle SDR: {}", e);
                    return 1;
                }
            };

            if indexer.supports_hooks() {
                indexer.on_install(&full_path);
            }

            let json = generate_change_request(
                &full_path,
                &new_uuid,
                metadata.name.as_deref(),
                metadata.author.as_deref(),
                icon.as_deref(),
                false,
                indexer.mime_type(),
                indexer.cde_type(),
                if metadata.extra.is_empty() {
                    None
                } else {
                    Some(&metadata.extra)
                },
            );

            scanner.post_change(&json);
            0
        }
        ScannerEventType::AddThumb | ScannerEventType::UpdateThumb => 0,
    }
}

#[no_mangle]
pub extern "C" fn load_extractor(
    handler: *mut *mut ScannerEventHandler,
    unk1: *mut c_int,
) -> c_int {
    crate::log::init("/mnt/us/kompanion.log");
    log::info!("kompanion extractor v4.1.0 initialised");

    crate::indexer::init_registry();

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
