//! Team banners: what an upload has to be, and the copy of it the site keeps.
//!
//! An upload is never stored as sent. It's decoded and encoded again, so what the site
//! serves is only pixels: no EXIF (a phone photo's location), no text chunks, nothing
//! riding behind the image, and no size the header claims but browsers would have to pay
//! for. An animated GIF stays animated: its frames are written again as they are (same
//! palettes, pixels and timing) into a new file that has nothing else in it. An animated
//! WebP keeps its first frame.

use std::{
    collections::HashMap,
    io::Cursor,
    num::NonZeroU64,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use image::{
    codecs::{jpeg::JpegEncoder, png::PngEncoder},
    metadata::Orientation,
    DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits,
};
use uuid::Uuid;

use crate::error::TeamError;

// The largest upload. Mirrored by BANNER_MAX_BYTES in frontend/src/lib/format.ts.
pub const MAX_UPLOAD_BYTES: usize = 2 * 1024 * 1024;
// Banners are shown 3:1 and never wider than the overlay's 1920, so this is generous. The
// pixel count is what decoding costs: 4 bytes each, so 32 MB at the limit.
const MAX_SIDE: u32 = 4096;
const MAX_PIXELS: u64 = 4096 * 2048;
// What decoding may allocate besides the pixels themselves.
const MAX_DECODE_ALLOC: u64 = 128 * 1024 * 1024;
// A copy may come out larger than its upload (a lossy WebP kept as PNG for its
// transparency), but not by this much.
const MAX_STORED_BYTES: usize = 2 * MAX_UPLOAD_BYTES;
const JPEG_QUALITY: u8 = 90;
// An animation: a browser draws the whole canvas for every frame, so the two are capped
// together (a 1200x400 banner gets 133 frames, a 600x200 one all 300).
const MAX_FRAMES: u64 = 300;
const MAX_ANIMATION_PIXELS: u64 = 64_000_000;
// How often one team can upload: each upload is decoded, rewrites the stored image and
// has Discord update the team's match cards.
const UPLOAD_COOLDOWN: Duration = Duration::from_secs(15);

/// The copy the site stores and serves.
#[derive(Debug)]
pub struct Stored {
    pub content_type: &'static str,
    pub data: Vec<u8>,
}

// Trusts the file's magic bytes rather than the request's Content-Type, and only lets
// these four formats near a decoder.
fn format_of(data: &[u8]) -> Option<ImageFormat> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(ImageFormat::Png)
    } else if data.starts_with(b"\xff\xd8\xff") {
        Some(ImageFormat::Jpeg)
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some(ImageFormat::Gif)
    } else if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        Some(ImageFormat::WebP)
    } else {
        None
    }
}

fn within_limits(width: u32, height: u32) -> bool {
    width <= MAX_SIDE && height <= MAX_SIDE && u64::from(width) * u64::from(height) <= MAX_PIXELS
}

/// Checks an upload and makes the copy to store. CPU-bound (it decodes and encodes the
/// image), so async callers run it with `spawn_blocking`.
pub fn process(data: &[u8]) -> Result<Stored, TeamError> {
    let format = format_of(data).ok_or(TeamError::BadBanner)?;
    let reader = || ImageReader::with_format(Cursor::new(data), format);

    // The size comes from the header alone, so an image too large is refused before any
    // of it is decoded.
    let (width, height) = reader()
        .into_dimensions()
        .map_err(|_| TeamError::BadBanner)?;
    if !within_limits(width, height) {
        return Err(TeamError::BannerTooLarge);
    }

    if format == ImageFormat::Gif {
        if let Some(stored) = animation(data)? {
            return Ok(stored);
        }
    }

    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    let mut reader = reader();
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|_| TeamError::BadBanner)?;
    // The copy has no EXIF to say which way is up, so a rotated photo is turned here.
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder).map_err(|_| TeamError::BadBanner)?;
    image.apply_orientation(orientation);

    let stored = encode(image, format)?;
    if stored.data.len() > MAX_STORED_BYTES {
        return Err(TeamError::BannerTooLarge);
    }
    Ok(stored)
}

fn animation_within_limits(width: u16, height: u16, frames: u64) -> bool {
    frames <= MAX_FRAMES && u64::from(width) * u64::from(height) * frames <= MAX_ANIMATION_PIXELS
}

// An animated GIF's copy, or None for a GIF of one frame (kept as a still image). Every
// frame is decoded to its colour indices, which proves the file is what it says, and
// written into a new GIF with its own palette, place, timing and transparency. Comments,
// application blocks other than the loop count and whatever follows the image aren't
// read, so they aren't copied.
fn animation(data: &[u8]) -> Result<Option<Stored>, TeamError> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::Indexed);
    options.set_memory_limit(gif::MemoryLimit::Bytes(
        NonZeroU64::new(MAX_PIXELS).expect("not zero"),
    ));
    // A frame has to lie within the canvas, so the canvas bounds what each one costs.
    options.check_frame_consistency(true);
    let mut decoder = options
        .read_info(Cursor::new(data))
        .map_err(|_| TeamError::BadBanner)?;
    let (width, height) = (decoder.width(), decoder.height());

    let mut out = Vec::new();
    let mut frames = 0;
    {
        let palette = decoder.global_palette().unwrap_or_default().to_vec();
        let mut encoder = gif::Encoder::new(&mut out, width, height, &palette)
            .map_err(|_| TeamError::BadBanner)?;
        encoder
            .set_repeat(decoder.repeat())
            .map_err(|_| TeamError::BadBanner)?;
        while let Some(frame) = decoder
            .read_next_frame()
            .map_err(|_| TeamError::BadBanner)?
        {
            frames += 1;
            if !animation_within_limits(width, height, frames) {
                return Err(TeamError::BannerTooLarge);
            }
            let mut frame = frame.clone();
            // A frame that waits for a key press would never move on in a browser.
            frame.needs_user_input = false;
            encoder
                .write_frame(&frame)
                .map_err(|_| TeamError::BadBanner)?;
        }
    }
    if frames < 2 {
        return Ok(None);
    }
    if out.len() > MAX_STORED_BYTES {
        return Err(TeamError::BannerTooLarge);
    }
    Ok(Some(Stored {
        content_type: "image/gif",
        data: out,
    }))
}

// Photos (JPEG, opaque WebP) are kept as JPEG; drawings (PNG, a GIF of one frame) and anything with
// transparency as PNG, in 8 bits a channel.
fn encode(image: DynamicImage, source: ImageFormat) -> Result<Stored, TeamError> {
    let alpha = image.color().has_alpha();
    let lossless = alpha || matches!(source, ImageFormat::Png | ImageFormat::Gif);
    let mut data = Vec::new();
    let (content_type, result) = if lossless {
        let image = if alpha {
            DynamicImage::ImageRgba8(image.into_rgba8())
        } else {
            DynamicImage::ImageRgb8(image.into_rgb8())
        };
        (
            "image/png",
            image.write_with_encoder(PngEncoder::new(&mut data)),
        )
    } else {
        let image = DynamicImage::ImageRgb8(image.into_rgb8());
        let encoder = JpegEncoder::new_with_quality(&mut data, JPEG_QUALITY);
        ("image/jpeg", image.write_with_encoder(encoder))
    };
    result.map_err(|_| TeamError::BadBanner)?;
    Ok(Stored { content_type, data })
}

/// When each team last uploaded a banner. In memory: a restart forgets it, which only
/// lets one upload through early.
#[derive(Clone, Default)]
pub struct Cooldowns(Arc<Mutex<HashMap<Uuid, Instant>>>);

impl Cooldowns {
    /// Allows an upload unless the team made one within the cooldown.
    pub fn take(&self, team_id: Uuid) -> bool {
        let mut uploads = self.0.lock().unwrap();
        let now = Instant::now();
        uploads.retain(|_, at| now.duration_since(*at) < UPLOAD_COOLDOWN);
        if uploads.contains_key(&team_id) {
            return false;
        }
        uploads.insert(team_id, now);
        true
    }
}

#[cfg(test)]
mod tests {
    use image::{
        codecs::gif::{GifDecoder, GifEncoder, Repeat},
        AnimationDecoder, Delay, Frame, Rgb, RgbImage, Rgba, RgbaImage,
    };

    use super::*;

    fn sample(width: u32, height: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_fn(width, height, |x, y| {
            Rgb([(x * 7) as u8, (y * 11) as u8, 90])
        }))
    }

    fn bytes(image: &DynamicImage, format: ImageFormat) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        image.write_to(&mut out, format).unwrap();
        out.into_inner()
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    fn decoded(stored: &Stored) -> DynamicImage {
        image::load_from_memory(&stored.data).unwrap()
    }

    #[test]
    fn a_png_stays_a_png_with_the_same_pixels() {
        let image = sample(60, 20);
        let stored = process(&bytes(&image, ImageFormat::Png)).unwrap();
        assert_eq!(stored.content_type, "image/png");
        assert_eq!(decoded(&stored).into_rgb8(), image.into_rgb8());
    }

    #[test]
    fn a_jpeg_stays_a_jpeg() {
        let stored = process(&bytes(&sample(60, 20), ImageFormat::Jpeg)).unwrap();
        assert_eq!(stored.content_type, "image/jpeg");
        assert!(stored.data.starts_with(b"\xff\xd8\xff"));
        let image = decoded(&stored);
        assert_eq!((image.width(), image.height()), (60, 20));
    }

    #[test]
    fn transparency_is_kept() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 8, Rgba([10, 20, 30, 0])));
        let stored = process(&bytes(&image, ImageFormat::WebP)).unwrap();
        assert_eq!(stored.content_type, "image/png");
        assert_eq!(decoded(&stored).into_rgba8(), image.into_rgba8());
    }

    #[test]
    fn what_rides_behind_the_image_is_dropped() {
        let mut upload = bytes(&sample(60, 20), ImageFormat::Png);
        upload.extend_from_slice(b"<script>alert(1)</script>");
        let stored = process(&upload).unwrap();
        assert!(!contains(&stored.data, b"<script>"));
    }

    #[test]
    fn exif_is_dropped() {
        let jpeg = bytes(&sample(60, 20), ImageFormat::Jpeg);
        // An APP1 segment right after the start-of-image marker, as cameras write it.
        let payload = b"Exif\0\0GPS-SECRET-LOCATION";
        let mut upload = jpeg[..2].to_vec();
        upload.extend_from_slice(&[0xff, 0xe1]);
        upload.extend_from_slice(&(payload.len() as u16 + 2).to_be_bytes());
        upload.extend_from_slice(payload);
        upload.extend_from_slice(&jpeg[2..]);
        assert!(contains(&upload, b"GPS-SECRET-LOCATION"));

        let stored = process(&upload).unwrap();
        assert!(!contains(&stored.data, b"GPS-SECRET-LOCATION"));
        assert!(!contains(&stored.data, b"Exif"));
    }

    // A GIF of 10x10 frames, one per shade, each shown for 100 ms.
    fn animated_gif(shades: impl IntoIterator<Item = u8>) -> Vec<u8> {
        let frames = shades.into_iter().map(|shade| {
            Frame::from_parts(
                RgbaImage::from_pixel(10, 10, Rgba([shade, shade, shade, 255])),
                0,
                0,
                Delay::from_numer_denom_ms(100, 1),
            )
        });
        let mut upload = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut upload);
            encoder.set_repeat(Repeat::Infinite).unwrap();
            encoder.encode_frames(frames).unwrap();
        }
        upload
    }

    fn frames_of(gif: &[u8]) -> Vec<Frame> {
        GifDecoder::new(Cursor::new(gif))
            .unwrap()
            .into_frames()
            .collect_frames()
            .unwrap()
    }

    #[test]
    fn an_animated_gif_stays_animated() {
        let upload = animated_gif([0, 120, 255]);
        let stored = process(&upload).unwrap();
        assert_eq!(stored.content_type, "image/gif");

        let (before, after) = (frames_of(&upload), frames_of(&stored.data));
        assert_eq!(after.len(), 3);
        for (before, after) in before.iter().zip(&after) {
            assert_eq!(before.buffer(), after.buffer());
            assert_eq!(before.delay(), after.delay());
        }
        // Still loops.
        assert!(contains(&stored.data, b"NETSCAPE2.0"));
    }

    #[test]
    fn an_animated_gif_loses_what_is_not_the_animation() {
        let mut upload = animated_gif([0, 255]);
        // A comment block before the trailer, and a payload after it.
        let trailer = upload.pop().unwrap();
        assert_eq!(trailer, 0x3b);
        upload.extend_from_slice(&[0x21, 0xfe, 11]);
        upload.extend_from_slice(b"GPS-COMMENT");
        upload.extend_from_slice(&[0, trailer]);
        upload.extend_from_slice(b"<script>alert(1)</script>");

        let stored = process(&upload).unwrap();
        assert_eq!(stored.content_type, "image/gif");
        assert_eq!(frames_of(&stored.data).len(), 2);
        assert!(!contains(&stored.data, b"GPS-COMMENT"));
        assert!(!contains(&stored.data, b"<script>"));
    }

    #[test]
    fn a_gif_of_one_frame_is_a_still_image() {
        let stored = process(&animated_gif([40])).unwrap();
        assert_eq!(stored.content_type, "image/png");
        assert_eq!(
            decoded(&stored).into_rgba8().get_pixel(5, 5).0,
            [40, 40, 40, 255]
        );
    }

    #[test]
    fn an_animation_too_long_is_refused() {
        let long = animated_gif((0..=MAX_FRAMES).map(|i| i as u8));
        assert_eq!(process(&long).unwrap_err(), TeamError::BannerTooLarge);
        // The larger the canvas, the fewer frames.
        assert!(animation_within_limits(600, 200, MAX_FRAMES));
        assert!(animation_within_limits(1200, 400, 133));
        assert!(!animation_within_limits(1200, 400, 134));
    }

    #[test]
    fn a_gif_cut_off_is_refused() {
        let upload = animated_gif([0, 120, 255]);
        assert_eq!(
            process(&upload[..upload.len() - 20]).unwrap_err(),
            TeamError::BadBanner
        );
    }

    #[test]
    fn an_image_too_large_is_refused() {
        let wide = bytes(&sample(MAX_SIDE + 1, 1), ImageFormat::Png);
        assert_eq!(process(&wide).unwrap_err(), TeamError::BannerTooLarge);
        // Each side fits, the pixel count doesn't.
        assert!(!within_limits(MAX_SIDE, MAX_SIDE));
        assert!(within_limits(MAX_SIDE, 2048));
        assert!(within_limits(1500, 500));
    }

    #[test]
    fn a_file_that_only_starts_like_an_image_is_refused() {
        let mut fake = b"\x89PNG\r\n\x1a\n".to_vec();
        fake.extend_from_slice(b"MZ this is not an image at all");
        assert_eq!(process(&fake).unwrap_err(), TeamError::BadBanner);
        assert_eq!(process(b"GIF89a").unwrap_err(), TeamError::BadBanner);

        // Cut off half way: the header reads, the pixels don't.
        let png = bytes(&sample(200, 200), ImageFormat::Png);
        assert_eq!(
            process(&png[..png.len() / 2]).unwrap_err(),
            TeamError::BadBanner
        );
    }

    #[test]
    fn other_formats_are_refused() {
        assert_eq!(process(b"").unwrap_err(), TeamError::BadBanner);
        assert_eq!(
            process(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>").unwrap_err(),
            TeamError::BadBanner
        );
        assert_eq!(process(b"BM\0\0\0\0").unwrap_err(), TeamError::BadBanner);
    }

    #[test]
    fn a_team_uploads_once_per_cooldown() {
        let cooldowns = Cooldowns::default();
        let (a, b) = (Uuid::from_u128(1), Uuid::from_u128(2));
        assert!(cooldowns.take(a));
        assert!(!cooldowns.take(a));
        assert!(cooldowns.take(b));
    }
}
