use std::fs::File;
use std::io::{self, BufReader, Cursor};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use image::codecs::{png, qoi};
use image::{guess_format, DynamicImage, ImageError, ImageFormat, ImageReader, RgbaImage};

pub fn write_png(writer: impl io::Write, image: &RgbaImage) -> image::ImageResult<()> {
    let encoder = png::PngEncoder::new_with_quality(
        writer,
        png::CompressionType::Best,
        Default::default()
    );
    image.write_with_encoder(encoder)
}

pub fn encode_cache_qoi(image: &RgbaImage) -> image::ImageResult<Vec<u8>> {
    let mut dest = vec![];
    let encoder = qoi::QoiEncoder::new(&mut dest);
    image.write_with_encoder(encoder)?;
    Ok(dest)
}

pub fn decode_cache_qoi(data: &[u8]) -> anyhow::Result<RgbaImage> {
    let decoder = qoi::QoiDecoder::new(data)?;
    let image = DynamicImage::from_decoder(decoder)?;
    match image {
        DynamicImage::ImageRgba8(v) => Ok(v),
        _ => anyhow::bail!("Decoded cached image is wrong pixel format"),
    }
}

pub fn load_image(path: impl AsRef<Path>) -> image::ImageResult<DynamicImage> {
    _load_image(path.as_ref())
}

pub fn read_file_and_load_image(path: impl AsRef<Path>) -> image::ImageResult<DynamicImage> {
    let path = path.as_ref();
    let file = std::fs::read(path)?;
    load_image_from_memory(&file, path)
}

fn _load_image(path: &Path) -> image::ImageResult<DynamicImage> {
    // use slightly larger buffer to optimize readahead interaction
    let reader = BufReader::with_capacity(32768, File::open(path)?);
    let reader = ImageReader::new(reader)
        .with_guessed_format()?;
    reader.decode()
}

/// Load image from memory, determining the format from the magic bytes or the file extension
pub fn load_image_from_memory(bytes: &[u8], path: &Path) -> image::ImageResult<DynamicImage> {
    let mut reader = ImageReader::new(Cursor::new(bytes));
    reader.set_format(
        guess_format(bytes)
            .or_else(|_| ImageFormat::from_path(path) )?
    );
    reader.decode()
}

const ADAPTIVE_THRES: u64 = 32*1024*1024;
const ADAPTIVE_BUF_SIZE: u64 = 4*1024*1024;

pub fn load_image_adaptive(path: impl AsRef<Path>) -> image::ImageResult<DynamicImage> {
    _load_image_adaptive(path.as_ref())
}

fn _load_image_adaptive(path: &Path) -> image::ImageResult<DynamicImage> {
    let metadata = path.metadata()?;

    if !metadata.is_file() {
        return Err(ImageError::IoError(io::Error::from(io::ErrorKind::IsADirectory)));
    }

    if metadata.size() > ADAPTIVE_THRES {
        _load_image(path)
    } else {
        read_file_and_load_image(path)
    }
}

pub fn load_image_off_thread(path: impl AsRef<Path>) -> anyhow::Result<DynamicImage> {
    let path = path.as_ref();
    let result = std::thread::scope(|s| {
        s.spawn(|| _load_image(path) ).join()
    });
    match result {
        Ok(v) => Ok(v?),
        Err(e) => {
            std::thread::spawn(|| std::panic::resume_unwind(e) );
            anyhow::bail!("Image decoding panicked");
        }
    }
}
