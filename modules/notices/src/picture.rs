//! A notice's own picture, as an avatar or a thumbnail: the image it was
//! sent with, never its program's icon, which its mark says. One sent as
//! pixels (`image-data`) is written to a file of its own, so the card and
//! the history both find it as a file, where the shell's server could only
//! keep it while the notice was open. One named by path is taken as it is,
//! and one given as an icon's name, or the same as its program's icon, is
//! an icon, so the notice has no picture of its own (VKCF3Z, 2026-11-07).

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

/// Pixels as the specification's `image-data` hint holds them,
/// `(iiibiiay)`: width, height, bytes a row, whether there's alpha, bits a
/// sample, samples a pixel, and the rows.
#[derive(Debug)]
pub struct Pixels {
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub alpha: bool,
    pub bits: i32,
    pub channels: i32,
    pub data: Vec<u8>,
}

/// The picture a path names, as a `file://` address: a file's path or
/// address that isn't its program's icon. A name with no folder is an
/// icon's.
pub fn named(path: &str, icon: &str) -> String {
    if path.is_empty() || path == icon {
        return String::new();
    }
    if let Some(file) = path.strip_prefix("file://") {
        return if file == icon {
            String::new()
        } else {
            path.to_owned()
        };
    }
    if path.starts_with('/') {
        return format!("file://{path}");
    }
    String::new()
}

/// Where a notice's pixels are written in `folder`, by its id and when
/// it came, so one said again in place is a file of its own.
pub fn file_for(folder: &Path, id: u32, at: i64) -> PathBuf {
    folder.join(format!("{id}-{at}.png"))
}

/// Writes pixels as a PNG to `file`, answering its `file://` address, or
/// none where they don't hold the picture they say: eight bits a sample,
/// three samples or four with alpha, and every row there.
pub fn write(pixels: &Pixels, file: &Path) -> Option<String> {
    let width = u32::try_from(pixels.width)
        .ok()
        .filter(|width| *width > 0)?;
    let height = u32::try_from(pixels.height)
        .ok()
        .filter(|height| *height > 0)?;
    let stride = usize::try_from(pixels.stride).ok()?;
    let channels = usize::try_from(pixels.channels).ok()?;
    let (color, wanted) = match (pixels.alpha, channels) {
        (true, 4) => (png::ColorType::Rgba, 4),
        (false, 3) => (png::ColorType::Rgb, 3),
        _ => return None,
    };
    if pixels.bits != 8 {
        return None;
    }
    let row = usize::try_from(width).ok()? * wanted;
    let rows = usize::try_from(height).ok()?;
    if stride < row || pixels.data.len() < stride * (rows - 1) + row {
        return None;
    }
    let mut packed = Vec::with_capacity(row * rows);
    for at in 0..rows {
        packed.extend_from_slice(&pixels.data[at * stride..at * stride + row]);
    }
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).ok()?;
    }
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(file).ok()?), width, height);
    encoder.set_color(color);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(&packed).ok()?;
    writer.finish().ok()?;
    Some(format!("file://{}", file.display()))
}

/// Takes away a picture this daemon wrote, once its notice is no longer
/// kept; one its sender named stays, since it's the sender's.
pub fn forget(picture: &str, folder: &Path) {
    if let Some(file) = picture.strip_prefix("file://").map(Path::new)
        && file.starts_with(folder)
    {
        let _ = std::fs::remove_file(file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("notices-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder");
        folder
    }

    #[test]
    fn a_path_is_its_picture_but_an_icon_isnt() {
        assert_eq!(
            named("/home/someone/shot.png", ""),
            "file:///home/someone/shot.png"
        );
        assert_eq!(named("file:///tmp/a.png", ""), "file:///tmp/a.png");
        assert_eq!(named("mail-unread", ""), "");
        assert_eq!(
            named("/usr/share/icons/app.png", "/usr/share/icons/app.png"),
            ""
        );
        assert_eq!(named("file:///icons/app.png", "/icons/app.png"), "");
        assert_eq!(named("", ""), "");
    }

    #[test]
    fn pixels_are_written_as_a_png_its_rows_unpadded() {
        let folder = folder("pixels");
        // Two by two, red, green, blue and white, each row padded to eight bytes.
        let pixels = Pixels {
            width: 2,
            height: 2,
            stride: 8,
            alpha: false,
            bits: 8,
            channels: 3,
            data: vec![255, 0, 0, 0, 255, 0, 9, 9, 0, 0, 255, 255, 255, 255, 9, 9],
        };
        let file = file_for(&folder, 4, 99);
        let written = write(&pixels, &file).expect("it's written");
        assert_eq!(written, format!("file://{}", file.display()));
        let decoder = png::Decoder::new(std::io::BufReader::new(
            File::open(&file).expect("it's there"),
        ));
        let mut reader = decoder.read_info().expect("a PNG");
        let mut read = vec![0; reader.output_buffer_size().expect("a size")];
        reader.next_frame(&mut read).expect("its frame");
        assert_eq!(read, [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]);
        forget(&written, &folder);
        assert!(!file.exists(), "its own picture goes with it");
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn pixels_that_dont_hold_what_they_say_are_no_picture() {
        let folder = folder("short");
        let short = Pixels {
            width: 2,
            height: 2,
            stride: 6,
            alpha: true,
            bits: 8,
            channels: 4,
            data: vec![0; 10],
        };
        assert_eq!(write(&short, &file_for(&folder, 1, 1)), None);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_picture_its_sender_named_is_never_taken_away() {
        let folder = folder("named");
        let theirs = folder.with_extension("theirs.png");
        std::fs::write(&theirs, b"theirs").expect("written");
        forget(&format!("file://{}", theirs.display()), &folder);
        assert!(theirs.exists());
        let _ = std::fs::remove_file(&theirs);
        let _ = std::fs::remove_dir_all(&folder);
    }
}
