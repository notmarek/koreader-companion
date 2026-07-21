use std::sync::Mutex;

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ScannerEventType {
    Add = 0,
    Delete = 1,
    Update = 2,
    AddThumb = 3,
    UpdateThumb = 4,
}

impl ScannerEventType {
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::Add),
            1 => Some(Self::Delete),
            2 => Some(Self::Update),
            3 => Some(Self::AddThumb),
            4 => Some(Self::UpdateThumb),
            _ => None,
        }
    }
}

pub trait ScanManager: Send + Sync {
    fn post_change(&self, json: &Value) -> i32;
    fn gen_uuid(&self, out: &mut [u8; 37]);
    fn get_thumbnail_for_uuid(&self, uuid: &str) -> Option<String>;
    fn update_ccat(&self, uuid: &str, thumbnail_path: &str);
    fn delete_ccat_entry(&self, uuid: &str);
    fn get_sha1_hash(&self, data: &str) -> String;
}

pub static SCANNER: std::sync::OnceLock<Box<dyn ScanManager + 'static>> = std::sync::OnceLock::new();

#[cfg(all(feature = "real-scanner", not(test), not(target_arch = "x86_64")))]
fn default_scanner_manager() -> Box<dyn ScanManager + 'static> {
    Box::new(RealScanner)
}

#[cfg(any(not(feature = "real-scanner"), test, target_arch = "x86_64"))]
fn default_scanner_manager() -> Box<dyn ScanManager + 'static> {
    Box::new(MockScanManager::default())
}

pub fn get_scanner_manager() -> &'static (dyn ScanManager + 'static) {
    SCANNER
        .get_or_init(default_scanner_manager)
        .as_ref()
}

pub fn set_scanner_manager(mgr: Box<dyn ScanManager + 'static>) {
    let _ = SCANNER.set(mgr);
}

pub struct MockScanManager {
    pub posted_changes: Mutex<Vec<String>>,
    pub deleted_entries: Mutex<Vec<String>>,
    pub updated_ccat: Mutex<Vec<(String, String)>>,
}

impl Default for MockScanManager {
    fn default() -> Self {
        Self {
            posted_changes: Mutex::new(Vec::new()),
            deleted_entries: Mutex::new(Vec::new()),
            updated_ccat: Mutex::new(Vec::new()),
        }
    }
}

impl ScanManager for MockScanManager {
    fn post_change(&self, json: &Value) -> i32 {
        let s = json.to_string();
        eprintln!("[MOCK scanner] post_change: {}", &s[..s.len().min(200)]);
        self.posted_changes.lock().unwrap().push(s);
        0
    }

    fn gen_uuid(&self, out: &mut [u8; 37]) {
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let s = format!("{:032}-{:04}", n, n % 10000);
        let bytes = s.as_bytes();
        let len = bytes.len().min(36);
        out[..len].copy_from_slice(&bytes[..len]);
        for i in len..36 {
            out[i] = b'0';
        }
        out[36] = 0;
    }

    fn get_thumbnail_for_uuid(&self, _uuid: &str) -> Option<String> {
        Some("/path/to/thumbnail.png".to_string())
    }

    fn update_ccat(&self, uuid: &str, thumbnail_path: &str) {
        eprintln!("[MOCK scanner] update_ccat: {} -> {}", uuid, thumbnail_path);
        self.updated_ccat
            .lock()
            .unwrap()
            .push((uuid.to_string(), thumbnail_path.to_string()));
    }

    fn delete_ccat_entry(&self, uuid: &str) {
        eprintln!("[MOCK scanner] delete_ccat_entry: {}", uuid);
        self.deleted_entries.lock().unwrap().push(uuid.to_string());
    }

    fn get_sha1_hash(&self, _data: &str) -> String {
        "0000000000000000000000000000000000000000".to_string()
    }
}

#[cfg(all(feature = "real-scanner", not(test), not(target_arch = "x86_64")))]
mod raw {
    use std::os::raw::{c_char, c_int, c_void};

    #[link(name = "scanner")]
    extern "C" {
        pub fn scanner_post_change(json: *mut c_void) -> c_int;
        pub fn scanner_gen_uuid(out: *mut c_char, buffer_size: c_int);
        pub fn scanner_get_thumbnail_for_uuid(uuid: *mut c_char) -> *mut c_char;
        pub fn scanner_update_ccat(uuid: *mut c_char, thumbnail_path: *mut c_char);
        pub fn scanner_delete_ccat_entry(uuid: *mut c_char);
        pub fn getSha1Hash(data: *const c_char) -> *mut c_char;
    }
}

#[cfg(all(feature = "real-scanner", not(test), not(target_arch = "x86_64")))]
pub fn set_real_scanner_manager() {
    set_scanner_manager(Box::new(RealScanner));
}

#[cfg(all(feature = "real-scanner", not(test), not(target_arch = "x86_64")))]
struct RealScanner;

#[cfg(all(feature = "real-scanner", not(test), not(target_arch = "x86_64")))]
impl ScanManager for RealScanner {
    fn post_change(&self, json: &Value) -> i32 {
        let json_str = json.to_string();
        let c_str = std::ffi::CString::new(json_str).unwrap();
        unsafe { raw::scanner_post_change(c_str.as_ptr() as *mut _) }
    }

    fn gen_uuid(&self, out: &mut [u8; 37]) {
        let mut buf = [0u8; 37];
        unsafe { raw::scanner_gen_uuid(buf.as_mut_ptr() as *mut _, 37) };
        *out = buf;
    }

    fn get_thumbnail_for_uuid(&self, uuid: &str) -> Option<String> {
        let c_str = std::ffi::CString::new(uuid).unwrap();
        let ptr = unsafe { raw::scanner_get_thumbnail_for_uuid(c_str.as_ptr() as *mut _) };
        if ptr.is_null() {
            None
        } else {
            let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() };
            Some(s)
        }
    }

    fn update_ccat(&self, uuid: &str, thumbnail_path: &str) {
        let c_uuid = std::ffi::CString::new(uuid).unwrap();
        let c_path = std::ffi::CString::new(thumbnail_path).unwrap();
        unsafe {
            raw::scanner_update_ccat(c_uuid.as_ptr() as *mut _, c_path.as_ptr() as *mut _);
        }
    }

    fn delete_ccat_entry(&self, uuid: &str) {
        let c_str = std::ffi::CString::new(uuid).unwrap();
        unsafe { raw::scanner_delete_ccat_entry(c_str.as_ptr() as *mut _) };
    }

    fn get_sha1_hash(&self, data: &str) -> String {
        let c_str = std::ffi::CString::new(data).unwrap();
        let ptr = unsafe { raw::getSha1Hash(c_str.as_ptr()) };
        if ptr.is_null() {
            String::new()
        } else {
            unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() }
        }
    }
}
