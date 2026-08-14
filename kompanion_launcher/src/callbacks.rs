use std::sync::Mutex;

use kompanion_sys::lipc::{LIPCcode, LipcManager, LIPC};

const SERVICE_NAME: &str = "com.notmarek.kompanion.launcher";

pub struct LauncherState {
    pub app_pid: i32,
    pub should_exit: bool,
}

impl Default for LauncherState {
    fn default() -> Self {
        Self {
            app_pid: -1,
            should_exit: false,
        }
    }
}

pub static STATE: Mutex<LauncherState> = Mutex::new(LauncherState {
    app_pid: -1,
    should_exit: false,
});

pub fn stub_reply(
    lipc_mgr: &(dyn LipcManager + 'static),
    lipc: *mut LIPC,
    property: &str,
    value: &str,
) -> LIPCcode {
    log::debug!("Stub called for \"{}\" with value \"{}\"", property, value);

    let id = value.split(':').next().unwrap_or("0");
    let response = format!("{}:0:", id);
    let target = format!("{}result", property);
    log::debug!("Replying with {} = {}", target, response);

    lipc_mgr.set_string_property(lipc, "com.lab126.appmgrd", &target, &response);
    LIPCcode::Ok
}

pub fn parse_go_value(value: &str) -> Option<String> {
    let after_colon = value.find(':')?;
    let after_prefix = &value[after_colon + 1..];

    let app_prefix = format!("app://{}/", SERVICE_NAME);
    let path_part = after_prefix.strip_prefix(&app_prefix)?;

    let file_path = if let Some(q) = path_part.find('?') {
        &path_part[..q]
    } else {
        path_part
    };

    Some(file_path.to_string())
}

pub fn build_launch_command(file_path: &str) -> String {
    format!("/mnt/us/koreader/koreader.sh --asap \"{}\"", file_path)
}

pub fn spawn_app(command: &str) -> Result<i32, String> {
    log::info!("Spawning app with command: {}", command);
    let command_str = format!("/var/local/mkk/su -c '{}'", command);
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(&command_str)
        .spawn()
    {
        Ok(child) => Ok(child.id() as i32),
        Err(e) => Err(format!("Failed to spawn: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kompanion_sys::lipc::MockLipcManager;

    #[test]
    fn test_stub_reply() {
        let mock = Box::new(MockLipcManager::default());
        let lipc = std::ptr::null_mut();
        let result = stub_reply(mock.as_ref(), lipc, "load", "abc123:some_value");
        assert_eq!(result, LIPCcode::Ok);
        let calls = mock.set_string_calls.lock().unwrap();
        assert!(!calls.is_empty());
        assert_eq!(calls[0].0, "com.lab126.appmgrd");
        assert_eq!(calls[0].1, "loadresult");
        assert!(calls[0].2.starts_with("abc123:"));
    }

    #[test]
    fn test_stub_reply_no_colon() {
        let mock = Box::new(MockLipcManager::default());
        let lipc = std::ptr::null_mut();
        let result = stub_reply(mock.as_ref(), lipc, "go", "simple_value");
        assert_eq!(result, LIPCcode::Ok);
    }

    #[test]
    fn test_parse_go_value_basic() {
        let value = format!("N:app://{}/./mnt/us/scripts/test.sh", SERVICE_NAME);
        let result = parse_go_value(&value);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "./mnt/us/scripts/test.sh");
    }

    #[test]
    fn test_parse_go_value_with_query() {
        let value = format!("N:app://{}/./mnt/us/scripts/test.sh?param=1", SERVICE_NAME);
        let result = parse_go_value(&value);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "./mnt/us/scripts/test.sh");
    }

    #[test]
    fn test_url_decode_with_path() {
        let decoded = percent_encoding::percent_decode(b"%2Fmnt%2Fus%2Fdocuments%2Ftest.sh")
            .decode_utf8_lossy()
            .into_owned();
        assert_eq!(decoded, "/mnt/us/documents/test.sh");
    }

    #[test]
    fn test_build_launch_command_simple() {
        let cmd = build_launch_command("/mnt/us/documents/book.epub");
        assert_eq!(cmd, "/mnt/us/koreader/koreader.sh --asap \"/mnt/us/documents/book.epub\"");
    }

    #[test]
    fn test_build_launch_command_spaces_in_filename() {
        // Regression: single-quoted path broke for filenames with spaces — the shell
        // closed the outer su -c '...' quote at the first space, splitting the filename
        // into separate words. koreader.sh then received only the first word as the path.
        let cmd = build_launch_command("/mnt/us/documents/Wie du die Affen in deinem Kopf zähmst.epub");
        assert!(cmd.contains("\""), "path must be double-quoted to survive spaces in su -c '...'");
        assert!(cmd.contains("Wie du die Affen in deinem Kopf zähmst.epub"), "full filename must be preserved");
        assert!(!cmd.contains("'"), "no single quotes in command — spawn_app wraps in single quotes");
    }

    #[test]
    fn test_build_launch_command_umlaut() {
        let cmd = build_launch_command("/mnt/us/documents/Bücher/käfer.epub");
        assert_eq!(cmd, "/mnt/us/koreader/koreader.sh --asap \"/mnt/us/documents/Bücher/käfer.epub\"");
    }
}
