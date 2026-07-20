fn hex_decode(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

pub fn url_decode(raw: &str) -> String {
    let raw = raw.as_bytes();
    let mut result = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%' && i + 2 < raw.len() {
            let hi = hex_decode(raw[i + 1]);
            let lo = hex_decode(raw[i + 2]);
            result.push((hi << 4) | lo);
            i += 3;
        } else {
            result.push(raw[i]);
            i += 1;
        }
    }
    String::from_utf8(result).unwrap_or_default()
}

pub fn strip(s: &str) -> String {
    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_decode_simple() {
        assert_eq!(url_decode("hello%20world"), "hello world");
    }

    #[test]
    fn test_url_decode_plain() {
        assert_eq!(url_decode("simple/path.sh"), "simple/path.sh");
    }

    #[test]
    fn test_url_decode_special_chars() {
        assert_eq!(url_decode("%2Fmnt%2Fus%2Fdocuments%2Ftest.sh"), "/mnt/us/documents/test.sh");
    }

    #[test]
    fn test_hex_decode_all() {
        for (i, expected) in (0u8..16).enumerate() {
            let hex_char = if i < 10 {
                b'0' + i as u8
            } else {
                b'A' + (i - 10) as u8
            };
            assert_eq!(hex_decode(hex_char), expected);
        }
        for (i, expected) in (0u8..16).enumerate() {
            let hex_char = if i < 10 {
                b'0' + i as u8
            } else {
                b'a' + (i - 10) as u8
            };
            assert_eq!(hex_decode(hex_char), expected);
        }
    }

    #[test]
    fn test_strip() {
        assert_eq!(strip("  hello  "), "hello");
        assert_eq!(strip("\t\nfoo\r\n"), "foo");
        assert_eq!(strip("no_whitespace"), "no_whitespace");
    }
}
