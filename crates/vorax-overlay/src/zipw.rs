//! A small .zip writer for the toolbox's problem reports (not in upstream): files stored as they are (the
//! pictures are PNGs, already compressed), UTF-8 names. Enough for Windows' own "Extract all" and GitHub.

use std::io::{self, Write};

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    let mut crc = !0u32;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    !crc
}

/// Writes a zip holding `files` (name in the zip, contents).
pub fn write<W: Write>(out: &mut W, files: &[(String, Vec<u8>)]) -> io::Result<()> {
    let mut central = Vec::new();
    let mut offset: u32 = 0;
    for (name, data) in files {
        let crc = crc32(data);
        let size = data.len() as u32;
        let name = name.as_bytes();
        // local file header: stored, UTF-8 name (bit 11), no date
        let mut head = Vec::with_capacity(30 + name.len());
        head.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        head.extend_from_slice(&20u16.to_le_bytes());
        head.extend_from_slice(&0x0800u16.to_le_bytes());
        head.extend_from_slice(&0u16.to_le_bytes());
        head.extend_from_slice(&0u16.to_le_bytes());
        head.extend_from_slice(&0x0021u16.to_le_bytes());
        head.extend_from_slice(&crc.to_le_bytes());
        head.extend_from_slice(&size.to_le_bytes());
        head.extend_from_slice(&size.to_le_bytes());
        head.extend_from_slice(&(name.len() as u16).to_le_bytes());
        head.extend_from_slice(&0u16.to_le_bytes());
        head.extend_from_slice(name);
        out.write_all(&head)?;
        out.write_all(data)?;
        // its central directory entry
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0x0800u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0x0021u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0u8; 12]);
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name);
        offset += head.len() as u32 + size;
    }
    out.write_all(&central)?;
    let mut end = Vec::with_capacity(22);
    end.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    end.extend_from_slice(&[0u8; 4]);
    end.extend_from_slice(&(files.len() as u16).to_le_bytes());
    end.extend_from_slice(&(files.len() as u16).to_le_bytes());
    end.extend_from_slice(&(central.len() as u32).to_le_bytes());
    end.extend_from_slice(&offset.to_le_bytes());
    end.extend_from_slice(&0u16.to_le_bytes());
    out.write_all(&end)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_known_crc() {
        assert_eq!(super::crc32(b"123456789"), 0xCBF4_3926);
    }

    /// The zip opens with an ordinary unzip and gives the files back (where `unzip` is installed).
    #[test]
    fn a_report_zip_unpacks() {
        let dir = std::env::temp_dir().join(format!("zipw-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let zip = dir.join("t.zip");
        let files = vec![("記錄/overlay.log".to_string(), b"hello\n".to_vec()), ("a.png".to_string(), vec![0u8, 1, 2, 255])];
        super::write(&mut std::fs::File::create(&zip).unwrap(), &files).unwrap();
        let Ok(out) = std::process::Command::new("unzip").arg("-o").arg("-q").arg(&zip).arg("-d").arg(&dir).output() else { return };
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(std::fs::read(dir.join("記錄/overlay.log")).unwrap(), b"hello\n");
        assert_eq!(std::fs::read(dir.join("a.png")).unwrap(), vec![0u8, 1, 2, 255]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
