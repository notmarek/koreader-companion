use std::os::raw::{c_char, c_void};

use koreader_companion_sys::lipc::{get_lipc_manager, LIPCcode, LIPC};

use crate::callbacks::{self, STATE};

const SERVICE_NAME: &str = "github.koreader.companion.launcher";

extern "C" fn stub_callback(
    lipc: *mut LIPC,
    property: *const c_char,
    value: *mut c_void,
    _data: *mut c_void,
) -> LIPCcode {
    let property_str = unsafe { std::ffi::CStr::from_ptr(property).to_string_lossy() };
    let value_str = unsafe { std::ffi::CStr::from_ptr(value as *const c_char).to_string_lossy() };
    let mgr = get_lipc_manager();
    callbacks::stub_reply(mgr, lipc, &property_str, &value_str)
}

extern "C" fn pause_ccb(
    lipc: *mut LIPC,
    property: *const c_char,
    value: *mut c_void,
    data: *mut c_void,
) -> LIPCcode {
    stub_callback(lipc, property, value, data)
}

extern "C" fn unload_ccb(
    lipc: *mut LIPC,
    property: *const c_char,
    value: *mut c_void,
    data: *mut c_void,
) -> LIPCcode {
    eprintln!("unload_callback");

    {
        let state = STATE.lock().unwrap();
        if state.app_pid > 0 {
            let command_str = format!("/var/local/mkk/su -c \"kill -9 {}\"", state.app_pid);
            eprintln!("Killing with: {}", command_str);
            let _ = std::process::Command::new("sh")
                .arg("-c")
                .arg(&command_str)
                .output();
        }
    }

    let result = stub_callback(lipc, property, value, data);
    STATE.lock().unwrap().should_exit = true;
    result
}

extern "C" fn go_ccb(
    lipc: *mut LIPC,
    property: *const c_char,
    value: *mut c_void,
    _data: *mut c_void,
) -> LIPCcode {
    let property_str = unsafe { std::ffi::CStr::from_ptr(property).to_string_lossy() };
    let value_str = unsafe { std::ffi::CStr::from_ptr(value as *const c_char).to_string_lossy() };

    eprintln!("go_callback");

    let mgr = get_lipc_manager();

    let file_path = match callbacks::parse_go_value(&value_str) {
        Some(p) => p,
        None => {
            return callbacks::stub_reply(mgr, lipc, &property_str, &value_str);
        }
    };

    let decoded_path = url::form_urlencoded::parse(file_path.as_bytes())
        .next()
        .map(|(k, _)| k.into_owned())
        .unwrap_or_default();
    eprintln!("Decoded path: \"{}\"", decoded_path);

    let _ = filetime::set_file_mtime(&decoded_path, filetime::FileTime::now());

    mgr.set_int_property(lipc, "com.lab126.scanner", "doFullScan", 1);

    let command = match callbacks::get_script_command_from_path(&decoded_path) {
        Some(c) => c,
        None => {
            eprintln!("Could not get script command!");
            return callbacks::stub_reply(mgr, lipc, &property_str, &value_str);
        }
    };

    eprintln!("Invoking app using \"{}\"", command);

    match callbacks::spawn_app(&command) {
        Ok(pid) => {
            STATE.lock().unwrap().app_pid = pid;
        }
        Err(e) => {
            eprintln!("Failed to spawn app: {}", e);
        }
    }

    callbacks::stub_reply(mgr, lipc, &property_str, &value_str)
}

pub fn run_main() -> i32 {
    eprintln!("koreader_companion launcher v4.1.0");

    let mgr = get_lipc_manager();
    let (lipc_opt, code) = mgr.open_ex(SERVICE_NAME);

    if code != LIPCcode::Ok {
        return 1;
    }

    let lipc = match lipc_opt {
        Some(l) => l,
        None => return 1,
    };

    eprintln!("Registering properties");

    mgr.register_string_property(
        lipc,
        "load",
        None,
        Some(stub_callback),
        std::ptr::null_mut(),
    );
    mgr.register_string_property(lipc, "unload", None, Some(unload_ccb), std::ptr::null_mut());
    mgr.register_string_property(lipc, "pause", None, Some(pause_ccb), std::ptr::null_mut());
    mgr.register_string_property(lipc, "go", None, Some(go_ccb), std::ptr::null_mut());

    mgr.set_string_property(
        lipc,
        "com.lab126.appmgrd",
        "runresult",
        &format!("0:{}", SERVICE_NAME),
    );

    eprintln!("Waiting to exit...");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));

        if STATE.lock().unwrap().should_exit {
            break;
        }

        let app_pid = STATE.lock().unwrap().app_pid;
        if app_pid > 0 {
            eprintln!("Child spawned, waiting to quit");
            let _ = wait_for_child(app_pid);

            let mut state = STATE.lock().unwrap();
            state.app_pid = -1;

            eprintln!("Exiting");
            mgr.set_string_property(lipc, "com.lab126.appmgrd", "stop", SERVICE_NAME);
            let _ = std::process::Command::new("/usr/bin/xrefresh")
                .arg("-d")
                .arg(":0.0")
                .output();
        }
    }

    eprintln!("Running exit routine");
    mgr.close(lipc);
    0
}

fn wait_for_child(pid: i32) -> i32 {
    unsafe {
        let mut status: i32 = 0;
        libc::waitpid(pid, &mut status, 0);
        status
    }
}
