// Ports platform/default/src/mln/util/image.cpp

use mln_util::image::PremultipliedImage;

/// `mln::decodeImage(const std::string&)`: dispatches on the magic bytes.
pub fn decode_image(data: &[u8]) -> Result<PremultipliedImage, String> {
    let read_uint = |offset: usize| -> u32 {
        let ptr = &data[offset..offset + 4];
        (u32::from(ptr[0]) << 24)
            | (u32::from(ptr[1]) << 16)
            | (u32::from(ptr[2]) << 8)
            | u32::from(ptr[3])
    };

    let size = data.len();

    if size >= 16 {
        let magic1 = read_uint(0x0);
        let magic2 = read_uint(0x8);
        // RIFF <xxxx = file size> WEBP
        if magic1 == 0x52494646 && magic2 == 0x57454250 {
            // TODO(port): decodeWEBP (platform/default/src/mln/util/webp_reader.cpp).
            return Err("TODO(port): WebP decoding is not ported".to_string());
        }
    }

    if size >= 4 {
        let magic = read_uint(0x0);
        if magic == 0x89504E47 {
            return crate::png_reader::decode_png(data);
        }
    }

    if size >= 2 {
        let magic = ((u16::from(data[0]) << 8) | u16::from(data[1])) & 0xffff;
        if magic == 0xFFD8 {
            // TODO(port): decodeJPEG (platform/default/src/mln/util/jpeg_reader.cpp).
            return Err("TODO(port): JPEG decoding is not ported".to_string());
        }
    }

    Err("unsupported image type".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported() {
        assert_eq!(decode_image(b"").unwrap_err(), "unsupported image type");
        assert_eq!(decode_image(b"GIF89a").unwrap_err(), "unsupported image type");
    }
}

#[cfg(test)]
mod fixture_tests {
    // Ports the fixture cases of test/util/image.test.cpp.
    use super::*;

    fn read_file(path: &str) -> Vec<u8> {
        let root = std::env::var_os("RUNFILES_DIR")
            .or_else(|| std::env::var_os("TEST_SRCDIR"))
            .expect("runfiles");
        let path = std::path::Path::new(&root).join("maplibre+").join(path);
        std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    // TEST(Image, PNGReadNoProfile)
    #[test]
    fn png_read_no_profile() {
        let image = decode_image(&read_file("test/fixtures/image/no_profile.png")).unwrap();
        assert_eq!(&image.data[..4], &[128, 0, 0, 255]);
    }

    // TEST(Image, PNGReadNoProfileAlpha)
    #[test]
    fn png_read_no_profile_alpha() {
        let image = decode_image(&read_file("test/fixtures/image/no_profile_alpha.png")).unwrap();
        assert_eq!(&image.data[..4], &[64, 0, 0, 128]);
    }

    // TEST(Image, PNGReadProfile)
    #[test]
    fn png_read_profile() {
        let image = decode_image(&read_file("test/fixtures/image/profile.png")).unwrap();
        assert_eq!(&image.data[..4], &[128, 0, 0, 255]);
    }

    // TEST(Image, PNGReadProfileAlpha)
    #[test]
    fn png_read_profile_alpha() {
        let image = decode_image(&read_file("test/fixtures/image/profile_alpha.png")).unwrap();
        assert_eq!(&image.data[..4], &[64, 0, 0, 128]);
    }

    // TEST(Image, PNGTile)
    #[test]
    fn png_tile() {
        let image = decode_image(&read_file("test/fixtures/image/tile.png")).unwrap();
        assert_eq!(image.size.width, 256);
        assert_eq!(image.size.height, 256);
    }

    // TEST(Image, JPEGTile)
    #[test]
    #[ignore = "TODO(port): decodeJPEG (platform/default/src/mln/util/jpeg_reader.cpp)"]
    fn jpeg_tile() {
        let image = decode_image(&read_file("test/fixtures/image/tile.jpeg")).unwrap();
        assert_eq!(image.size.width, 256);
        assert_eq!(image.size.height, 256);
    }

    // TEST(Image, WebPTile)
    #[test]
    #[ignore = "TODO(port): decodeWEBP (platform/default/src/mln/util/webp_reader.cpp)"]
    fn webp_tile() {
        let image = decode_image(&read_file("test/fixtures/image/tile.webp")).unwrap();
        assert_eq!(image.size.width, 256);
        assert_eq!(image.size.height, 256);
    }

    // Not in the C++ suite: JPEG and WebP are recognized and reported as not
    // ported rather than as unsupported.
    #[test]
    fn jpeg_and_webp_are_todo() {
        for file in ["tile.jpeg", "tile.webp"] {
            let error = decode_image(&read_file(&format!("test/fixtures/image/{file}"))).unwrap_err();
            assert!(error.starts_with("TODO(port)"), "{file}: {error}");
        }
    }
}
