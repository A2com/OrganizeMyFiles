//! Minimal ZIP writer (stored entries, no compression).
//! Needed for the "compress" action of the rule engine.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

pub fn zip_file(src: &Path, out: &Path) -> io::Result<u64> {
    let name = src.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "item".to_string());
    let entries = vec![(src.to_path_buf(), name)];
    write_zip(&entries, out)
}

pub fn zip_dir(src: &Path, out: &Path) -> io::Result<u64> {
    let mut entries = Vec::new();
    collect(src, src, &mut entries)?;
    write_zip(&entries, out)
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(std::path::PathBuf, String)>) -> io::Result<()> {
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let p = e.path();
        // Les liens ne sont pas suivis : un lien qui remonte au dossier parent ferait boucler la compression.
        if std::fs::symlink_metadata(&p)?.file_type().is_symlink() {
            continue;
        }
        if p.is_dir() {
            collect(root, &p, out)?;
        } else {
            let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
            out.push((p, rel));
        }
    }
    Ok(())
}

fn write_zip(entries: &[(std::path::PathBuf, String)], out: &Path) -> io::Result<u64> {
    let mut f = File::create(out)?;
    let mut central: Vec<u8> = Vec::new();
    let mut offset = 0u32;
    let mut total = 0u64;

    for (path, name) in entries {
        let mut data = Vec::new();
        File::open(path)?.read_to_end(&mut data)?;
        let crc = crc32(&data);
        let name_b = name.as_bytes();
        let size = data.len() as u32;

        let mut local = Vec::new();
        local.extend_from_slice(&0x04034b50u32.to_le_bytes()); // local header sig
        local.extend_from_slice(&20u16.to_le_bytes()); // version needed
        local.extend_from_slice(&0u16.to_le_bytes()); // flags
        local.extend_from_slice(&0u16.to_le_bytes()); // method: store
        local.extend_from_slice(&0u16.to_le_bytes()); // mod time
        local.extend_from_slice(&0u16.to_le_bytes()); // mod date
        local.extend_from_slice(&crc.to_le_bytes());
        local.extend_from_slice(&size.to_le_bytes());
        local.extend_from_slice(&size.to_le_bytes());
        local.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes()); // extra len
        local.extend_from_slice(name_b);
        local.extend_from_slice(&data);
        f.write_all(&local)?;

        central.extend_from_slice(&0x02014b50u32.to_le_bytes()); // central sig
        central.extend_from_slice(&20u16.to_le_bytes()); // made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra
        central.extend_from_slice(&0u16.to_le_bytes()); // comment
        central.extend_from_slice(&0u16.to_le_bytes()); // disk
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name_b);

        offset += local.len() as u32;
        total += data.len() as u64;
    }

    let cd_offset = offset;
    let cd_size = central.len() as u32;
    f.write_all(&central)?;
    let mut eocd = Vec::new();
    eocd.extend_from_slice(&0x06054b50u32.to_le_bytes());
    eocd.extend_from_slice(&0u16.to_le_bytes());
    eocd.extend_from_slice(&0u16.to_le_bytes());
    eocd.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    eocd.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    eocd.extend_from_slice(&cd_size.to_le_bytes());
    eocd.extend_from_slice(&cd_offset.to_le_bytes());
    eocd.extend_from_slice(&0u16.to_le_bytes());
    f.write_all(&eocd)?;
    Ok(total)
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("omf-zip-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Lance `zip_dir` dans un fil : un lien circulaire ferait boucler la version fautive.
    fn zip_dir_with_timeout(src: &Path, out: &Path) -> io::Result<u64> {
        let (src, out) = (src.to_path_buf(), out.to_path_buf());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(zip_dir(&src, &out));
        });
        rx.recv_timeout(Duration::from_secs(10)).expect("la compression ne doit pas boucler")
    }

    #[test]
    fn a_directory_is_written_with_its_files() {
        let dir = temp_dir("plain");
        std::fs::write(dir.join("a.txt"), b"abcde").unwrap();
        std::fs::create_dir(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/b.txt"), b"xy").unwrap();
        let out = dir.join("sortie.zip");
        let total = zip_dir_with_timeout(&dir, &out).unwrap();
        assert_eq!(total, 7, "seuls les fichiers réguliers sont compressés");
        let bytes = std::fs::read(&out).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("a.txt") && text.contains("sub/b.txt"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_followed() {
        use std::os::unix::fs::symlink;
        let dir = temp_dir("liens");
        std::fs::write(dir.join("a.txt"), b"abc").unwrap();
        std::fs::create_dir(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/b.txt"), b"de").unwrap();
        symlink("..", dir.join("sub/remonte")).unwrap();
        symlink(".", dir.join("ici")).unwrap();
        let out = dir.join("sortie.zip");
        let total = zip_dir_with_timeout(&dir, &out).unwrap();
        assert_eq!(total, 5, "les liens ne sont pas suivis");
        let bytes = std::fs::read(&out).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("remonte") && !text.contains("ici"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
