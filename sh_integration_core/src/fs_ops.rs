use std::path::Path;

pub fn rmdir_r(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        std::fs::remove_dir_all(path)
    } else {
        Ok(())
    }
}

pub fn copy_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, dst)?;
    Ok(())
}

pub fn path_exists(path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_rmdir_r() {
        let tmp = std::env::temp_dir().join("sh_integration_test_rmdir");
        std::fs::create_dir_all(tmp.join("subdir")).unwrap();
        std::fs::write(tmp.join("subdir").join("test.txt"), b"hello").unwrap();
        assert!(tmp.exists());
        rmdir_r(&tmp).unwrap();
        assert!(!tmp.exists());
    }

    #[test]
    fn test_rmdir_r_nonexistent() {
        let p = PathBuf::from("/tmp/sh_integration_nonexistent_xyz");
        rmdir_r(&p).unwrap();
    }
}
