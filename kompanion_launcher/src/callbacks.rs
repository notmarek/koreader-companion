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

// Turn the value from parse_go_value (e.g. "./mnt/us/documents/foo.epub", possibly
// percent-encoded for non-ASCII characters) into an absolute filesystem path.
// The value has no leading slash, so one is prepended after percent-decoding.
pub fn decode_go_path(file_path: &str) -> String {
    format!(
        "/{}",
        percent_encoding::percent_decode(file_path.as_bytes())
            .decode_utf8_lossy()
            .into_owned()
    )
}

pub fn build_launch_command(file_path: &str) -> String {
    format!("/mnt/us/koreader/koreader.sh --asap \"{}\"", file_path)
}

// Wrap `command` in a single-quoted `su_path -c '...'` invocation.
// Single quotes in the command are escaped via the '\'' idiom so filenames
// containing apostrophes (e.g. "The Webs' Archive.epub") survive the shell.
pub fn build_spawn_command(su_path: &str, command: &str) -> String {
    let escaped = command.replace('\'', "'\\''");
    format!("{} -c '{}'", su_path, escaped)
}

pub fn spawn_app(command: &str) -> Result<i32, String> {
    log::info!("Spawning app with command: {}", command);
    let command_str = build_spawn_command("/var/local/mkk/su", command);
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

    // Run build_spawn_command through a real sh, substituting `printf '%s\n'` for
    // su. printf prints each argument on its own line; line 0 is "-c", line 1 is
    // the inner command string — i.e. exactly what su would have received.
    fn inner_via_shell(file_path: &str) -> String {
        let launch_cmd = build_launch_command(file_path);
        let spawn_cmd = build_spawn_command("printf '%s\\n'", &launch_cmd);
        let output = std::process::Command::new("sh")
            .args(["-c", &spawn_cmd])
            .output()
            .expect("sh must be available");
        assert!(
            output.status.success(),
            "shell rejected spawn command for {:?}: {}",
            file_path,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.lines().nth(1).unwrap_or("").to_string()
    }

    #[test]
    fn test_build_launch_command_umlaut() {
        let cmd = build_launch_command("/mnt/us/documents/Bücher/käfer.epub");
        assert_eq!(cmd, "/mnt/us/koreader/koreader.sh --asap \"/mnt/us/documents/Bücher/käfer.epub\"");
    }

    #[test]
    fn test_build_launch_command_arabic_epub() {
        // Real-world filename: Arabic title with [Arabic] tag, author, year, URL slug,
        // ISBN, hash, and source tag — all separated by " -- "
        let path = "/mnt/us/documents/الرمز المفقود، طبعة خاصة مصورة [Arabic] -- دان براون -- 2009 -- https___t_me_mystery_books_ar -- isbn13 9780385533829 -- 4c09d7143957f731b60438ded778b988 -- The Webs' Archive.epub";
        let cmd = build_launch_command(path);
        assert!(cmd.contains("\""), "path must be double-quoted");
        assert!(cmd.contains("الرمز المفقود"), "Arabic title preserved");
        assert!(cmd.contains("[Arabic]"), "bracketed tag preserved");
        assert!(cmd.contains("دان براون"), "Arabic author preserved");
        assert!(cmd.contains("isbn13 9780385533829"), "ISBN preserved");
        assert!(cmd.contains("4c09d7143957f731b60438ded778b988"), "hash preserved");
        assert!(cmd.contains("The Webs' Archive.epub"), "source tag with apostrophe preserved");
    }

    #[test]
    fn test_build_launch_command_arabic_mobi() {
        // Real-world .mobi filename: Arabic author with comma, full publisher name with
        // commas and city, year — all in the filename
        let path = "/mnt/us/documents/براون، دان - جحيم [Arabic] -- دان براون -- Penguin Random House LLC, New York, 2013 -- https___t_me_mystery_books_ar -- isbn13 9780385537858 -- 079c203c83fa25d6261722e930cdc824 -- The Webs' Archive.mobi";
        let cmd = build_launch_command(path);
        assert!(cmd.contains("براون، دان"), "Arabic author with Arabic comma preserved");
        assert!(cmd.contains("Penguin Random House LLC, New York, 2013"), "publisher with commas and year preserved");
        assert!(cmd.contains("The Webs' Archive.mobi"), ".mobi extension and source tag preserved");
    }

    #[test]
    fn test_build_launch_command_apostrophe_in_path() {
        // build_launch_command double-quotes the path, so an apostrophe in the filename
        // is safe at this level. Note: spawn_app wraps the whole command in single quotes,
        // so a literal ' in the path would break the su -c '...' shell invocation.
        let path = "/mnt/us/documents/The Webs' Archive.epub";
        let cmd = build_launch_command(path);
        assert!(cmd.contains("\""), "path must be double-quoted");
        assert!(cmd.contains("The Webs' Archive.epub"), "apostrophe passed through at build level");
    }

    #[test]
    fn test_url_decode_arabic_filename() {
        // Arabic filenames arrive URL-encoded over LIPC before being passed to
        // build_launch_command. Verify percent_decode round-trips the Arabic correctly.
        let encoded = b"%2Fmnt%2Fus%2Fdocuments%2F%D8%A7%D9%84%D8%B1%D9%85%D8%B2%20%D8%A7%D9%84%D9%85%D9%81%D9%82%D9%88%D8%AF.epub";
        let decoded = percent_encoding::percent_decode(encoded)
            .decode_utf8_lossy()
            .into_owned();
        assert_eq!(decoded, "/mnt/us/documents/الرمز المفقود.epub");
    }

    #[test]
    fn test_build_spawn_command_escapes_apostrophe() {
        let cmd = build_spawn_command("/bin/su", "koreader.sh --asap \"The Webs' Archive.epub\"");
        // '\'' is the standard sh idiom for a literal ' inside single-quoted strings
        assert!(cmd.contains("'\\''"), "apostrophe must be escaped with the '\\'' idiom");
    }

    #[test]
    fn test_shell_execution_umlaut() {
        let inner = inner_via_shell("/mnt/us/documents/Wie du die Affen in deinem Kopf zähmst_ 20 Monkey-Mind-Hacks -- Baumgartner, Paul Johannes.epub");
        assert!(inner.contains("Wie du die Affen in deinem Kopf zähmst"), "umlaut filename preserved through shell");
        assert!(inner.contains("Baumgartner, Paul Johannes"), "comma in author preserved");
    }

    #[test]
    fn test_shell_execution_arabic_epub() {
        let inner = inner_via_shell("/mnt/us/documents/الرمز المفقود، طبعة خاصة مصورة [Arabic] -- دان براون -- 2009 -- https___t_me_mystery_books_ar -- isbn13 9780385533829 -- 4c09d7143957f731b60438ded778b988 -- The Webs' Archive.epub");
        assert!(inner.contains("الرمز المفقود"), "Arabic title preserved through shell");
        assert!(inner.contains("isbn13 9780385533829"), "ISBN preserved");
        assert!(inner.contains("The Webs' Archive.epub"), "apostrophe in source tag survived shell quoting");
    }

    #[test]
    fn test_shell_execution_arabic_mobi() {
        let inner = inner_via_shell("/mnt/us/documents/براون، دان - جحيم [Arabic] -- دان براون -- Penguin Random House LLC, New York, 2013 -- https___t_me_mystery_books_ar -- isbn13 9780385537858 -- 079c203c83fa25d6261722e930cdc824 -- The Webs' Archive.mobi");
        assert!(inner.contains("Penguin Random House LLC, New York, 2013"), "commas in publisher preserved");
        assert!(inner.contains("The Webs' Archive.mobi"), ".mobi with apostrophe preserved");
    }

    #[test]
    fn test_shell_execution_arabic_unknown_author() {
        let inner = inner_via_shell("/mnt/us/documents/درجة ممارسة رؤساء الأقسام الأكاديمية في كليات الجامعة -- مجهول -- www_goldenshamela_com -- fe90b27bb3ac46c030ded0f30783a9a4 -- The Webs' Archive.epub");
        assert!(inner.contains("مجهول"), "Arabic unknown-author marker preserved");
        assert!(inner.contains("www_goldenshamela_com"), "URL slug preserved");
        assert!(inner.contains("The Webs' Archive.epub"), "apostrophe survived");
    }

    #[test]
    fn test_shell_execution_persian_pdf() {
        let inner = inner_via_shell("/mnt/us/documents/حساب دیفرانسیل و انتگرال و هندسه تحلیلی - جلد اول قسمت اول -- جورج توماس , راس فینی _ برگردان به پارسی از سیامک کاظمی، -- 1_1, 7 -- مرکز نشر دانش.pdf");
        assert!(inner.contains("حساب دیفرانسیل"), "Persian title preserved through shell");
        assert!(inner.contains("1_1, 7"), "version with underscore preserved");
        assert!(inner.contains("مرکز نشر دانش"), "Persian publisher preserved");
        assert!(inner.ends_with(".pdf\""), ".pdf extension at end of command");
    }

    #[test]
    fn test_build_launch_command_arabic_unknown_author() {
        // Arabic filename with مجهول (unknown author), URL slug in web-safe encoding
        // (underscores instead of dots/slashes), hash, source tag — no ISBN
        let path = "/mnt/us/documents/درجة ممارسة رؤساء الأقسام الأكاديمية في كليات الجامعة -- مجهول -- www_goldenshamela_com -- fe90b27bb3ac46c030ded0f30783a9a4 -- The Webs' Archive.epub";
        let cmd = build_launch_command(path);
        assert!(cmd.contains("درجة ممارسة"), "Arabic title preserved");
        assert!(cmd.contains("مجهول"), "Arabic unknown-author marker preserved");
        assert!(cmd.contains("www_goldenshamela_com"), "URL slug with underscores preserved");
        assert!(cmd.contains("fe90b27bb3ac46c030ded0f30783a9a4"), "hash preserved");
        assert!(cmd.contains("The Webs' Archive.epub"), "source tag preserved");
    }

    #[test]
    fn test_build_launch_command_persian_pdf() {
        // Persian (Farsi) filename: multiple authors separated by " , " and " _ " for
        // translator, version string with underscore (1_1), Persian publisher, .pdf extension
        let path = "/mnt/us/documents/حساب دیفرانسیل و انتگرال و هندسه تحلیلی - جلد اول قسمت اول -- جورج توماس , راس فینی _ برگردان به پارسی از سیامک کاظمی، -- 1_1, 7 -- مرکز نشر دانش.pdf";
        let cmd = build_launch_command(path);
        assert!(cmd.contains("حساب دیفرانسیل"), "Persian title preserved");
        assert!(cmd.contains("جورج توماس , راس فینی"), "multiple authors with comma preserved");
        assert!(cmd.contains("برگردان به پارسی از سیامک کاظمی،"), "translator notation preserved");
        assert!(cmd.contains("1_1, 7"), "version string with underscore preserved");
        assert!(cmd.contains("مرکز نشر دانش"), "Persian publisher preserved");
        assert!(cmd.ends_with(".pdf\""), ".pdf extension preserved");
    }

    #[test]
    fn test_decode_go_path_prepends_slash() {
        // parse_go_value strips the app:// prefix and leaves a slashless path;
        // decode_go_path must turn it back into an absolute path.
        assert_eq!(
            decode_go_path("mnt/us/documents/book.epub"),
            "/mnt/us/documents/book.epub"
        );
    }

    #[test]
    fn test_decode_go_path_percent_encoded_spaces() {
        assert_eq!(
            decode_go_path("mnt/us/documents/My%20Book.epub"),
            "/mnt/us/documents/My Book.epub"
        );
    }

    #[test]
    fn test_decode_go_path_percent_encoded_arabic() {
        // Arabic characters arrive percent-encoded (UTF-8 bytes) over LIPC.
        let encoded = "mnt/us/documents/%D8%A7%D9%84%D8%B1%D9%85%D8%B2%20%D8%A7%D9%84%D9%85%D9%81%D9%82%D9%88%D8%AF.epub";
        assert_eq!(
            decode_go_path(encoded),
            "/mnt/us/documents/الرمز المفقود.epub"
        );
    }

    #[test]
    fn test_parse_then_decode_pipeline() {
        // Full go-callback path pipeline: parse_go_value -> decode_go_path.
        // parse_go_value returns "./mnt/..." (leading dot from the URL), so the
        // decoded absolute path is "/./mnt/..." — a valid path the shell resolves.
        let value = format!("N:app://{}/./mnt/us/documents/My%20Book.epub", SERVICE_NAME);
        let parsed = parse_go_value(&value).unwrap();
        let decoded = decode_go_path(&parsed);
        assert_eq!(decoded, "/./mnt/us/documents/My Book.epub");
    }
}
