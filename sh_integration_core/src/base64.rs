pub fn decode_base64_to_file(icon_str: &str, output_path: &str) -> Result<(), String> {
    let data_part = if let Some(idx) = icon_str.find("data:image/") {
        &icon_str[idx + "data:image/".len()..]
    } else {
        return Err("Not a data URI".to_string());
    };

    let semicolon_pos = data_part.find(';').unwrap_or(data_part.len());
    let _mime_type = &data_part[..semicolon_pos];

    let comma_pos = data_part.find(',').ok_or("Missing comma in data URI")?;
    let b64_data = &data_part[comma_pos + 1..];

    let bytes = base64_decode_manual(b64_data)?;

    let parent = std::path::Path::new(output_path)
        .parent()
        .ok_or("Invalid output path")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
    std::fs::write(output_path, bytes).map_err(|e| format!("Failed to write: {}", e))?;

    Ok(())
}

fn base64_decode_manual(input: &str) -> Result<Vec<u8>, String> {
    let mut result = Vec::new();
    let mut current_byte: u32 = 0;
    let mut processed_bits: i32 = 0;

    for ch in input.bytes() {
        let value = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => continue,
        } as u32;

        current_byte |= (value << 2) >> processed_bits;
        let consumed_bits = if (8 - processed_bits) < 6 {
            8 - processed_bits
        } else {
            6
        };
        processed_bits += consumed_bits;

        if processed_bits >= 8 {
            result.push(current_byte as u8);
            current_byte = value << (2 + consumed_bits);
            processed_bits = 6 - consumed_bits;
        }
    }

    Ok(result)
}

pub fn get_icon_extension(icon_str: &str) -> Option<String> {
    if !icon_str.starts_with("data:image/") {
        return None;
    }
    let after_prefix = &icon_str["data:image/".len()..];
    let semicolon = after_prefix.find(';').unwrap_or(after_prefix.len());
    Some(after_prefix[..semicolon].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_decode_basic() {
        let input = "aGVsbG8=";
        let result = base64_decode_manual(input).unwrap();
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_base64_decode_hello_world() {
        let input = "aGVsbG8gd29ybGQ=";
        let result = base64_decode_manual(input).unwrap();
        assert_eq!(result, b"hello world");
    }

    #[test]
    fn test_get_icon_extension_png() {
        let ext = get_icon_extension("data:image/png;base64,iVBOR...");
        assert_eq!(ext, Some("png".to_string()));
    }

    #[test]
    fn test_get_icon_extension_jpeg() {
        let ext = get_icon_extension("data:image/jpeg;base64,/9j/...");
        assert_eq!(ext, Some("jpeg".to_string()));
    }

    #[test]
    fn test_get_icon_extension_non_data_uri() {
        let ext = get_icon_extension("/path/to/icon.png");
        assert_eq!(ext, None);
    }
}
