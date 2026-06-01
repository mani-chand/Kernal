 // Embed the tar archive directly into the kernel binary's read-only memory
    const RAMDISK_DATA: &[u8] = include_bytes!("../../ramdisk.tar");

    /// Returns an iterator over all files in the ramdisk
    pub fn list_files() -> impl Iterator<Item = (&'static str, &'static [u8])> {
        parse_tar(RAMDISK_DATA)
    }

    /// Reads a file's raw bytes by filename
    pub fn read_file(filename: &str) -> Option<&'static [u8]> {
        for (name, data) in list_files() {
            if name == filename {
                return Some(data);
            }
        }
        None
    }

    /// Simple standard TAR parser written from scratch
    fn parse_tar(data: &'static [u8]) -> impl Iterator<Item = (&'static str, &'static [u8])> {
        let mut offset = 0;
        core::iter::from_fn(move || {
            while offset + 512 <= data.len() {
                let header = &data[offset..offset + 512];

                // A block of all zeros indicates the end of the archive
                if header.iter().all(|&x| x == 0) {
                    break;
                }

                // Filename (First 100 bytes, null-terminated ASCII)
                let name_bytes = &header[0..100];
                let name_len = name_bytes.iter().position(|&x| x == 0).unwrap_or(100);
                let name = core::str::from_utf8(&name_bytes[..name_len]).unwrap_or("");

                // File Size (12 bytes starting at offset 124, encoded as octal string)
                let size_bytes = &header[124..136];
                let size_str = core::str::from_utf8(size_bytes).unwrap_or("").trim();
                let size = usize::from_str_radix(size_str, 8).unwrap_or(0);

                offset += 512; // Advance past the header block to the file content
                let file_data = &data[offset..offset + size];

                // File data in tar is padded to 512-byte blocks
                let padded_size = (size + 511) / 512 * 512;
                offset += padded_size;

                if !name.is_empty() {
                    return Some((name, file_data));
                }
            }
            None
        })
    }