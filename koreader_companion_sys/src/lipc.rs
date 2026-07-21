#[allow(unused_imports)]
use std::os::raw::{c_char, c_int, c_void};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LIPCcode {
    Ok = 0,
    ErrorUnknown = 1,
    ErrorInternal = 2,
    ErrorNoSuchSource = 3,
    ErrorOperationNotSupported = 4,
    ErrorOutOfMemory = 5,
    ErrorSubscriptionFailed = 6,
    ErrorNoSuchParam = 7,
    ErrorNoSuchProperty = 8,
    ErrorAccessNotAllowed = 9,
    ErrorBufferTooSmall = 10,
    ErrorInvalidHandle = 11,
    ErrorInvalidArg = 12,
    ErrorOperationNotAllowed = 13,
    ErrorParamsSizeExceeded = 14,
    ErrorTimedOut = 15,
    ErrorServiceNameTooLong = 16,
    ErrorDuplicateServiceName = 17,
    ErrorInitDbus = 18,
    PropErrorInvalidState = 0x100,
    PropErrorNotInitialized = 0x101,
    PropErrorInternal = 0x102,
}

#[repr(C)]
pub struct LIPC {
    _private: [u8; 0],
}

pub type LipcPropCallback =
    extern "C" fn(lipc: *mut LIPC, property: *const c_char, value: *mut c_void, data: *mut c_void) -> LIPCcode;

pub trait LipcManager: Send + Sync {
    fn open_ex(&self, service: &str) -> (Option<*mut LIPC>, LIPCcode);
    fn close(&self, lipc: *mut LIPC);
    fn register_string_property(
        &self,
        lipc: *mut LIPC,
        property: &str,
        getter: Option<LipcPropCallback>,
        setter: Option<LipcPropCallback>,
        data: *mut c_void,
    ) -> LIPCcode;
    fn set_string_property(&self, lipc: *mut LIPC, service: &str, property: &str, value: &str) -> LIPCcode;
    fn set_int_property(&self, lipc: *mut LIPC, service: &str, property: &str, value: i32) -> LIPCcode;
    fn get_string_property(&self, lipc: *mut LIPC, service: &str, property: &str) -> (String, LIPCcode);
    fn free_string(&self, string: *mut c_char);
}

pub static LIPC_MGR: std::sync::OnceLock<Box<dyn LipcManager + 'static>> = std::sync::OnceLock::new();

#[cfg(all(feature = "real-lipc", not(test), not(target_arch = "x86_64")))]
fn default_lipc_manager() -> Box<dyn LipcManager + 'static> {
    Box::new(RealLipc)
}

#[cfg(any(not(feature = "real-lipc"), test, target_arch = "x86_64"))]
fn default_lipc_manager() -> Box<dyn LipcManager + 'static> {
    Box::new(MockLipcManager::default())
}

pub fn get_lipc_manager() -> &'static (dyn LipcManager + 'static) {
    LIPC_MGR
        .get_or_init(default_lipc_manager)
        .as_ref()
}

pub fn set_lipc_manager(mgr: Box<dyn LipcManager + 'static>) {
    let _ = LIPC_MGR.set(mgr);
}

pub struct MockLipcManager {
    pub set_string_calls: Mutex<Vec<(String, String, String)>>,
    pub set_int_calls: Mutex<Vec<(String, String, i32)>>,
    pub registered_properties: Mutex<Vec<String>>,
}

impl Default for MockLipcManager {
    fn default() -> Self {
        Self {
            set_string_calls: Mutex::new(Vec::new()),
            set_int_calls: Mutex::new(Vec::new()),
            registered_properties: Mutex::new(Vec::new()),
        }
    }
}

impl LipcManager for MockLipcManager {
    fn open_ex(&self, _service: &str) -> (Option<*mut LIPC>, LIPCcode) {
        (Some(0x1 as *mut LIPC), LIPCcode::Ok)
    }

    fn close(&self, _lipc: *mut LIPC) {}

    fn register_string_property(
        &self,
        _lipc: *mut LIPC,
        property: &str,
        _getter: Option<LipcPropCallback>,
        _setter: Option<LipcPropCallback>,
        _data: *mut c_void,
    ) -> LIPCcode {
        eprintln!("[MOCK lipc] register_string_property: {}", property);
        self.registered_properties
            .lock()
            .unwrap()
            .push(property.to_string());
        LIPCcode::Ok
    }

    fn set_string_property(&self, _lipc: *mut LIPC, service: &str, property: &str, value: &str) -> LIPCcode {
        eprintln!("[MOCK lipc] set_string: {} {} = {}", service, property, value);
        self.set_string_calls
            .lock()
            .unwrap()
            .push((service.to_string(), property.to_string(), value.to_string()));
        LIPCcode::Ok
    }

    fn set_int_property(&self, _lipc: *mut LIPC, service: &str, property: &str, value: i32) -> LIPCcode {
        eprintln!("[MOCK lipc] set_int: {} {} = {}", service, property, value);
        self.set_int_calls
            .lock()
            .unwrap()
            .push((service.to_string(), property.to_string(), value));
        LIPCcode::Ok
    }

    fn get_string_property(&self, _lipc: *mut LIPC, _service: &str, _property: &str) -> (String, LIPCcode) {
        (String::new(), LIPCcode::Ok)
    }

    fn free_string(&self, _string: *mut c_char) {}
}

#[cfg(all(feature = "real-lipc", not(test), not(target_arch = "x86_64")))]
mod raw {
    use super::LIPC;
    use std::os::raw::{c_char, c_int, c_void};

    #[link(name = "lipc")]
    extern "C" {
        pub fn LipcOpenEx(service: *const c_char, code: *mut c_int) -> *mut LIPC;
        pub fn LipcClose(lipc: *mut LIPC);
        pub fn LipcRegisterStringProperty(
            lipc: *mut LIPC,
            property: *const c_char,
            getter: Option<super::LipcPropCallback>,
            setter: Option<super::LipcPropCallback>,
            data: *mut c_void,
        ) -> super::LIPCcode;
        pub fn LipcSetStringProperty(
            lipc: *mut LIPC,
            service: *const c_char,
            property: *const c_char,
            value: *const c_char,
        ) -> super::LIPCcode;
        pub fn LipcSetIntProperty(
            lipc: *mut LIPC,
            service: *const c_char,
            property: *const c_char,
            value: c_int,
        ) -> super::LIPCcode;
        pub fn LipcGetStringProperty(
            lipc: *mut LIPC,
            service: *const c_char,
            property: *const c_char,
            value: *mut *mut c_char,
        ) -> super::LIPCcode;
        pub fn LipcFreeString(string: *mut c_char);
    }
}

#[cfg(all(feature = "real-lipc", not(test), not(target_arch = "x86_64")))]
pub fn set_real_lipc_manager() {
    set_lipc_manager(Box::new(RealLipc));
}

#[cfg(all(feature = "real-lipc", not(test), not(target_arch = "x86_64")))]
struct RealLipc;

#[cfg(all(feature = "real-lipc", not(test), not(target_arch = "x86_64")))]
impl LipcManager for RealLipc {
    fn open_ex(&self, service: &str) -> (Option<*mut LIPC>, LIPCcode) {
        let c_service = std::ffi::CString::new(service).unwrap();
        let mut code: LIPCcode = LIPCcode::Ok;
        let lipc = unsafe { raw::LipcOpenEx(c_service.as_ptr(), &mut code as *mut _ as *mut c_int) };
        if lipc.is_null() {
            (None, code)
        } else {
            (Some(lipc), code)
        }
    }

    fn close(&self, lipc: *mut LIPC) {
        unsafe { raw::LipcClose(lipc) };
    }

    fn register_string_property(
        &self,
        lipc: *mut LIPC,
        property: &str,
        getter: Option<LipcPropCallback>,
        setter: Option<LipcPropCallback>,
        data: *mut c_void,
    ) -> LIPCcode {
        let c_property = std::ffi::CString::new(property).unwrap();
        unsafe { raw::LipcRegisterStringProperty(lipc, c_property.as_ptr(), getter, setter, data) }
    }

    fn set_string_property(&self, lipc: *mut LIPC, service: &str, property: &str, value: &str) -> LIPCcode {
        let c_service = std::ffi::CString::new(service).unwrap();
        let c_property = std::ffi::CString::new(property).unwrap();
        let c_value = std::ffi::CString::new(value).unwrap();
        unsafe { raw::LipcSetStringProperty(lipc, c_service.as_ptr(), c_property.as_ptr(), c_value.as_ptr()) }
    }

    fn set_int_property(&self, lipc: *mut LIPC, service: &str, property: &str, value: i32) -> LIPCcode {
        let c_service = std::ffi::CString::new(service).unwrap();
        let c_property = std::ffi::CString::new(property).unwrap();
        unsafe { raw::LipcSetIntProperty(lipc, c_service.as_ptr(), c_property.as_ptr(), value) }
    }

    fn get_string_property(&self, lipc: *mut LIPC, service: &str, property: &str) -> (String, LIPCcode) {
        let c_service = std::ffi::CString::new(service).unwrap();
        let c_property = std::ffi::CString::new(property).unwrap();
        let mut value: *mut c_char = std::ptr::null_mut();
        let code = unsafe {
            raw::LipcGetStringProperty(lipc, c_service.as_ptr(), c_property.as_ptr(), &mut value)
        };
        let s = if value.is_null() {
            String::new()
        } else {
            let s = unsafe { std::ffi::CStr::from_ptr(value).to_string_lossy().into_owned() };
            unsafe { raw::LipcFreeString(value) };
            s
        };
        (s, code)
    }

    fn free_string(&self, string: *mut c_char) {
        unsafe { raw::LipcFreeString(string) };
    }
}
