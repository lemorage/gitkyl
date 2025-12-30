//! File type detection for blob rendering.
//!
//! Provides four-phase file type classification:
//! 1. Image extension detection (fast path for known image types)
//! 2. Image magic byte detection (reliable for extensionless files)
//! 3. NUL byte heuristic (git's binary detection approach)
//! 4. UTF-8 validation (text vs binary)

use std::path::Path;

/// Maximum bytes to check for NUL byte heuristic (git uses 8KB).
const BINARY_CHECK_LEN: usize = 8192;

/// File type classification for blob rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Text file suitable for syntax highlighting
    Text,
    /// Image file (raster or vector)
    Image(ImageFormat),
    /// Binary file not suitable for text display
    Binary,
}

/// Supported image formats for inline display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// Portable Network Graphics with transparency
    Png,
    /// JPEG compressed image (no transparency)
    Jpeg,
    /// Graphics Interchange Format with animation support
    Gif,
    /// Scalable Vector Graphics (XML based)
    Svg,
    /// WebP format with transparency and animation
    Webp,
    /// Bitmap image (uncompressed)
    Bmp,
    /// Icon format (multiple resolutions)
    Ico,
}

impl ImageFormat {
    /// MIME type for data URLs
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Svg => "image/svg+xml",
            Self::Webp => "image/webp",
            Self::Bmp => "image/bmp",
            Self::Ico => "image/x-icon",
        }
    }

    /// File extension without dot
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::Svg => "svg",
            Self::Webp => "webp",
            Self::Bmp => "bmp",
            Self::Ico => "ico",
        }
    }
}

/// Detects file type from path and content.
///
/// Uses four-phase detection for reliability:
/// 1. Image extension lookup (fast path)
/// 2. Image magic byte detection (handles extensionless files)
/// 3. NUL byte heuristic (git's binary detection approach)
/// 4. UTF-8 validation (valid UTF-8 = text, otherwise binary)
///
/// # Arguments
///
/// * `bytes`: File content bytes from git blob
/// * `path`: File path for extension checking
///
/// # Returns
///
/// Classified file type (Text, Image, or Binary)
///
/// # Examples
///
/// ```
/// use gitkyl::{detect_file_type, FileType, ImageFormat};
/// use std::path::Path;
///
/// let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
/// assert_eq!(
///     detect_file_type(&png_header, Path::new("test.png")),
///     FileType::Image(ImageFormat::Png)
/// );
///
/// let text = b"Hello, world!";
/// assert_eq!(detect_file_type(text, Path::new("test.txt")), FileType::Text);
/// ```
pub fn detect_file_type(bytes: &[u8], path: &Path) -> FileType {
    // Phase 1: Image detection by extension (fast path)
    if let Some(format) = detect_image_by_extension(path) {
        return FileType::Image(format);
    }

    // Phase 2: Image detection by magic bytes
    if let Some(format) = detect_image_by_magic(bytes) {
        return FileType::Image(format);
    }

    // Phase 3: NUL byte heuristic
    // If first 8KB contains NUL byte, it's binary
    let check_len = bytes.len().min(BINARY_CHECK_LEN);
    if bytes[..check_len].contains(&0) {
        return FileType::Binary;
    }

    // Phase 4: UTF-8 validation
    if std::str::from_utf8(bytes).is_ok() {
        FileType::Text
    } else {
        FileType::Binary
    }
}

/// Detects image format by file extension
fn detect_image_by_extension(path: &Path) -> Option<ImageFormat> {
    let ext = path.extension()?.to_str()?.to_lowercase();

    match ext.as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "gif" => Some(ImageFormat::Gif),
        "svg" => Some(ImageFormat::Svg),
        "webp" => Some(ImageFormat::Webp),
        "bmp" => Some(ImageFormat::Bmp),
        "ico" => Some(ImageFormat::Ico),
        _ => None,
    }
}

/// Detects image format from magic bytes
fn detect_image_by_magic(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.len() < 8 {
        return None;
    }

    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(ImageFormat::Png);
    }

    // JPEG: FF D8 FF
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(ImageFormat::Jpeg);
    }

    // GIF: GIF87a or GIF89a
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(ImageFormat::Gif);
    }

    // WebP: RIFF....WEBP
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some(ImageFormat::Webp);
    }

    // SVG: Root element must be <svg
    if is_svg_root_element(bytes) {
        return Some(ImageFormat::Svg);
    }

    // BMP: BM + valid file size in header
    if is_valid_bmp(bytes) {
        return Some(ImageFormat::Bmp);
    }

    // ICO: 00 00 01 00 + reasonable image count (1-20)
    if is_valid_ico(bytes) {
        return Some(ImageFormat::Ico);
    }

    None
}

/// Checks if content has SVG as root element.
///
/// Validates that `<svg` appears as the root element after optional
/// XML declaration and DOCTYPE, not just anywhere in the content.
fn is_svg_root_element(bytes: &[u8]) -> bool {
    let check_len = bytes.len().min(1024);
    let Ok(text) = std::str::from_utf8(&bytes[..check_len]) else {
        return false;
    };

    let mut content = text.trim_start();

    // Skip XML declaration: <?xml ... ?>
    if let Some(rest) = content.strip_prefix("<?xml") {
        if let Some(end) = rest.find("?>") {
            content = rest[end + 2..].trim_start();
        } else {
            return false;
        }
    }

    // Skip DOCTYPE: <!DOCTYPE ... >
    if let Some(rest) = content.strip_prefix("<!DOCTYPE") {
        if let Some(end) = rest.find('>') {
            content = rest[end + 1..].trim_start();
        } else {
            return false;
        }
    }

    // Root element must be <svg
    content.starts_with("<svg")
}

/// Validates BMP file structure.
///
/// BMP files start with "BM" followed by a 4-byte little-endian file size.
/// We validate that the size is at least the minimum header size (54 bytes).
fn is_valid_bmp(bytes: &[u8]) -> bool {
    if bytes.len() < 6 || !bytes.starts_with(b"BM") {
        return false;
    }

    let file_size = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);

    // 14 file header + 40 DIB header
    file_size >= 54
}

/// Validates ICO file structure.
///
/// ICO files start with 00 00 01 00 followed by image count (1-2 bytes).
/// Valid ICO files typically contain 1-20 images.
fn is_valid_ico(bytes: &[u8]) -> bool {
    if bytes.len() < 6 {
        return false;
    }

    if !bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
        return false;
    }

    let image_count = u16::from_le_bytes([bytes[4], bytes[5]]);

    (1..=20).contains(&image_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_png_by_extension() {
        let bytes = b"not actually png data";
        assert_eq!(
            detect_file_type(bytes, Path::new("test.png")),
            FileType::Image(ImageFormat::Png)
        );
    }

    #[test]
    fn test_detect_png_by_magic_bytes() {
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        assert_eq!(
            detect_file_type(&png_header, Path::new("unknown")),
            FileType::Image(ImageFormat::Png)
        );
    }

    #[test]
    fn test_detect_jpeg_by_extension() {
        let bytes = b"not actually jpeg data";
        assert_eq!(
            detect_file_type(bytes, Path::new("photo.jpg")),
            FileType::Image(ImageFormat::Jpeg)
        );
        assert_eq!(
            detect_file_type(bytes, Path::new("photo.jpeg")),
            FileType::Image(ImageFormat::Jpeg)
        );
    }

    #[test]
    fn test_detect_jpeg_by_magic_bytes() {
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0xFF, 0xDB];
        assert_eq!(
            detect_file_type(&jpeg_header, Path::new("unknown")),
            FileType::Image(ImageFormat::Jpeg)
        );
    }

    #[test]
    fn test_detect_gif_by_magic_bytes() {
        let gif87 = b"GIF87a\x00\x00";
        assert_eq!(
            detect_file_type(gif87, Path::new("unknown")),
            FileType::Image(ImageFormat::Gif)
        );

        let gif89 = b"GIF89a\x00\x00";
        assert_eq!(
            detect_file_type(gif89, Path::new("unknown")),
            FileType::Image(ImageFormat::Gif)
        );
    }

    #[test]
    fn test_detect_svg_by_extension() {
        let svg = b"<svg></svg>";
        assert_eq!(
            detect_file_type(svg, Path::new("icon.svg")),
            FileType::Image(ImageFormat::Svg)
        );
    }

    #[test]
    fn test_detect_svg_direct_tag() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        assert_eq!(
            detect_file_type(svg, Path::new("unknown")),
            FileType::Image(ImageFormat::Svg)
        );
    }

    #[test]
    fn test_detect_svg_with_xml_declaration() {
        let svg = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><svg></svg>";
        assert_eq!(
            detect_file_type(svg, Path::new("unknown")),
            FileType::Image(ImageFormat::Svg)
        );
    }

    #[test]
    fn test_detect_svg_with_xml_and_doctype() {
        let svg = b"<?xml version=\"1.0\"?><!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\"><svg></svg>";
        assert_eq!(
            detect_file_type(svg, Path::new("unknown")),
            FileType::Image(ImageFormat::Svg)
        );
    }

    #[test]
    fn test_detect_svg_with_whitespace() {
        let svg = b"  \n\t<?xml version=\"1.0\"?>\n  <svg></svg>";
        assert_eq!(
            detect_file_type(svg, Path::new("unknown")),
            FileType::Image(ImageFormat::Svg)
        );
    }

    #[test]
    fn test_detect_text_for_xml_plist() {
        let plist = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict></dict></plist>";
        assert_eq!(
            detect_file_type(plist, Path::new("theme.tmTheme")),
            FileType::Text
        );
    }

    #[test]
    fn test_detect_text_for_html_with_embedded_svg() {
        let html = b"<!DOCTYPE html><html><body><svg></svg></body></html>";
        assert_eq!(
            detect_file_type(html, Path::new("page.html")),
            FileType::Text
        );
    }

    #[test]
    fn test_detect_text_for_xml_config() {
        let xml = b"<?xml version=\"1.0\"?><configuration><setting name=\"foo\"/></configuration>";
        assert_eq!(
            detect_file_type(xml, Path::new("config.xml")),
            FileType::Text
        );
    }

    #[test]
    fn test_detect_text_for_markdown_mentioning_svg() {
        let md = b"# SVG Guide\n\nTo create an SVG, use `<svg>` tags.";
        assert_eq!(detect_file_type(md, Path::new("guide.md")), FileType::Text);
    }

    #[test]
    fn test_detect_webp_by_magic_bytes() {
        let webp = b"RIFF\x00\x00\x00\x00WEBP\x00\x00";
        assert_eq!(
            detect_file_type(webp, Path::new("unknown")),
            FileType::Image(ImageFormat::Webp)
        );
    }

    #[test]
    fn test_detect_webp_rejects_non_webp_riff() {
        let wav = b"RIFF\x00\x00\x00\x00WAVEfmt ";
        assert_eq!(detect_image_by_magic(wav), None);
    }

    #[test]
    fn test_detect_bmp_valid_header() {
        let bmp = b"BM\x36\x00\x00\x00\x00\x00\x00\x00";
        assert_eq!(
            detect_file_type(bmp, Path::new("unknown")),
            FileType::Image(ImageFormat::Bmp)
        );
    }

    #[test]
    fn test_detect_bmp_large_size() {
        let bmp = b"BM\xe8\x03\x00\x00\x00\x00\x00\x00";
        assert_eq!(
            detect_file_type(bmp, Path::new("unknown")),
            FileType::Image(ImageFormat::Bmp)
        );
    }

    #[test]
    fn test_detect_bmp_rejects_small_file_size() {
        assert!(!is_valid_bmp(b"BM\x10\x00\x00\x00"));
    }

    #[test]
    fn test_is_valid_bmp_rejects_short_input() {
        assert!(!is_valid_bmp(b"BM"));
        assert!(!is_valid_bmp(b"BM\x36\x00"));
    }

    #[test]
    fn test_is_valid_bmp_rejects_wrong_magic() {
        assert!(!is_valid_bmp(b"XX\x36\x00\x00\x00"));
    }

    #[test]
    fn test_detect_ico_valid_header() {
        let ico = [0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x20, 0x20];
        assert_eq!(
            detect_file_type(&ico, Path::new("unknown")),
            FileType::Image(ImageFormat::Ico)
        );
    }

    #[test]
    fn test_detect_ico_multiple_images() {
        let ico = [0x00, 0x00, 0x01, 0x00, 0x05, 0x00, 0x20, 0x20];
        assert_eq!(
            detect_file_type(&ico, Path::new("unknown")),
            FileType::Image(ImageFormat::Ico)
        );
    }

    #[test]
    fn test_detect_ico_rejects_zero_images() {
        let invalid = [0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x20, 0x20];
        assert_eq!(
            detect_file_type(&invalid, Path::new("unknown")),
            FileType::Binary
        );
    }

    #[test]
    fn test_detect_ico_rejects_too_many_images() {
        let invalid = [0x00, 0x00, 0x01, 0x00, 0x64, 0x00, 0x20, 0x20];
        assert_eq!(
            detect_file_type(&invalid, Path::new("unknown")),
            FileType::Binary
        );
    }

    #[test]
    fn test_image_extension_case_insensitive() {
        let bytes = b"data";
        assert_eq!(
            detect_file_type(bytes, Path::new("test.PNG")),
            FileType::Image(ImageFormat::Png)
        );
        assert_eq!(
            detect_file_type(bytes, Path::new("test.JPG")),
            FileType::Image(ImageFormat::Jpeg)
        );
        assert_eq!(
            detect_file_type(bytes, Path::new("test.GIF")),
            FileType::Image(ImageFormat::Gif)
        );
        assert_eq!(
            detect_file_type(bytes, Path::new("test.SVG")),
            FileType::Image(ImageFormat::Svg)
        );
    }

    #[test]
    fn test_detect_text_valid_utf8() {
        let text = b"Hello, world! This is valid UTF-8 text.";
        assert_eq!(
            detect_file_type(text, Path::new("readme.txt")),
            FileType::Text
        );
    }

    #[test]
    fn test_detect_text_unknown_extension() {
        let text = b"fn main() { println!(\"Hello\"); }";
        assert_eq!(detect_file_type(text, Path::new("code.rs")), FileType::Text);
        assert_eq!(
            detect_file_type(text, Path::new("unknown.xyz")),
            FileType::Text
        );
        assert_eq!(
            detect_file_type(text, Path::new("no_extension")),
            FileType::Text
        );
    }

    #[test]
    fn test_detect_text_unicode() {
        let unicode = "Hello 世界 🌍 Привет".as_bytes();
        assert_eq!(
            detect_file_type(unicode, Path::new("unicode.txt")),
            FileType::Text
        );
    }

    #[test]
    fn test_detect_text_empty_file() {
        let empty: &[u8] = b"";
        assert_eq!(
            detect_file_type(empty, Path::new("empty.txt")),
            FileType::Text
        );
    }

    #[test]
    fn test_no_extension() {
        let text = b"Makefile contents";
        assert_eq!(
            detect_file_type(text, Path::new("Makefile")),
            FileType::Text
        );
    }

    #[test]
    fn test_dotfile() {
        let text = b"gitignore contents";
        assert_eq!(
            detect_file_type(text, Path::new(".gitignore")),
            FileType::Text
        );
    }

    #[test]
    fn test_detect_binary_by_nul_byte() {
        let with_nul = b"some\x00binary\x00data";
        assert_eq!(
            detect_file_type(with_nul, Path::new("file.txt")),
            FileType::Binary
        );
    }

    #[test]
    fn test_detect_binary_nul_at_start() {
        let nul_start = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        assert_eq!(
            detect_file_type(&nul_start, Path::new("unknown")),
            FileType::Binary
        );
    }

    #[test]
    fn test_detect_binary_nul_in_middle() {
        let nul_middle = b"valid text\x00more text";
        assert_eq!(
            detect_file_type(nul_middle, Path::new("file.txt")),
            FileType::Binary
        );
    }

    #[test]
    fn test_detect_binary_invalid_utf8_no_nul() {
        let invalid_utf8 = [0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0xF9, 0xF8];
        assert_eq!(
            detect_file_type(&invalid_utf8, Path::new("unknown")),
            FileType::Binary
        );
    }

    #[test]
    fn test_detect_binary_typical_executable() {
        let elf_header = [0x7F, 0x45, 0x4C, 0x46, 0x02, 0x01, 0x01, 0x00];
        assert_eq!(
            detect_file_type(&elf_header, Path::new("program")),
            FileType::Binary
        );
    }

    #[test]
    fn test_detect_binary_typical_archive() {
        let zip_header = [0x50, 0x4B, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(
            detect_file_type(&zip_header, Path::new("archive.zip")),
            FileType::Binary
        );
    }

    #[test]
    fn test_image_magic_bytes_too_short() {
        let short = [0x00, 0x01, 0x02];
        assert_eq!(detect_image_by_magic(&short), None);
    }

    #[test]
    fn test_is_svg_root_element_rejects_incomplete_xml() {
        let incomplete = b"<?xml version=\"1.0\"<svg></svg>";
        assert!(!is_svg_root_element(incomplete));
    }

    #[test]
    fn test_is_svg_root_element_rejects_incomplete_doctype() {
        let incomplete = b"<?xml version=\"1.0\"?><!DOCTYPE svg<svg></svg>";
        assert!(!is_svg_root_element(incomplete));
    }

    #[test]
    fn test_is_svg_root_element_detects_direct_tag() {
        assert!(is_svg_root_element(b"<svg></svg>"));
        assert!(is_svg_root_element(b"<svg xmlns=\"...\">"));
        assert!(is_svg_root_element(b"  <svg>"));
    }

    #[test]
    fn test_is_svg_root_element_detects_with_xml_decl() {
        assert!(is_svg_root_element(b"<?xml version=\"1.0\"?><svg>"));
        assert!(is_svg_root_element(
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg>"
        ));
    }

    #[test]
    fn test_is_svg_root_element_rejects_non_svg_root() {
        assert!(!is_svg_root_element(b"<html><svg></svg></html>"));
        assert!(!is_svg_root_element(b"<?xml?><plist></plist>"));
        assert!(!is_svg_root_element(b"not xml at all"));
    }

    #[test]
    fn test_image_format_mime_types() {
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Gif.mime_type(), "image/gif");
        assert_eq!(ImageFormat::Svg.mime_type(), "image/svg+xml");
        assert_eq!(ImageFormat::Webp.mime_type(), "image/webp");
        assert_eq!(ImageFormat::Bmp.mime_type(), "image/bmp");
        assert_eq!(ImageFormat::Ico.mime_type(), "image/x-icon");
    }

    #[test]
    fn test_image_format_extensions() {
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Gif.extension(), "gif");
        assert_eq!(ImageFormat::Svg.extension(), "svg");
        assert_eq!(ImageFormat::Webp.extension(), "webp");
        assert_eq!(ImageFormat::Bmp.extension(), "bmp");
        assert_eq!(ImageFormat::Ico.extension(), "ico");
    }
}
