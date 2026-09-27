use std::path::PathBuf;

use image::{ImageDecoder, ImageError};
use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};

// --- ImageInfo
#[derive(Clone, Copy)]
pub struct ImageInfo {
	format:      image::ImageFormat,
	width:       u32,
	height:      u32,
	color:       image::ColorType,
	orientation: Option<image::metadata::Orientation>,
}

impl ImageInfo {
	pub async fn new(path: PathBuf) -> image::ImageResult<Self> {
		tokio::task::spawn_blocking(move || {
			let reader = image::ImageReader::open(path)?.with_guessed_format()?;

			let Some(format) = reader.format() else {
				return Err(ImageError::IoError(std::io::Error::new(
					std::io::ErrorKind::InvalidData,
					"unknown image format",
				)));
			};

			let mut decoder = reader.into_decoder()?;
			let (width, height) = decoder.dimensions();
			Ok(Self {
				format,
				width,
				height,
				color: decoder.color_type(),
				orientation: decoder.orientation().ok(),
			})
		})
		.await
		.map_err(|e| ImageError::IoError(e.into()))?
	}
}

impl UserData for ImageInfo {
	fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
		fields.add_field_method_get("w", |_, me| Ok(me.width));
		fields.add_field_method_get("h", |_, me| Ok(me.height));
		fields.add_field_method_get("ori", |_, me| Ok(me.orientation.map(|o| o.to_exif())));
		fields.add_field_method_get("format", |_, me| Ok(ImageFormat(me.format)));
		fields.add_field_method_get("color", |_, me| Ok(ImageColor(me.color)));
	}
}

// --- ImageFormat
struct ImageFormat(image::ImageFormat);

impl UserData for ImageFormat {
	fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
		methods.add_meta_method(MetaMethod::ToString, |_, me, ()| {
			use image::ImageFormat as F;

			Ok(match me.0 {
				F::Png => "PNG",
				F::Jpeg => "JPEG",
				F::Gif => "GIF",
				F::WebP => "WEBP",
				F::Pnm => "PNM",
				F::Tiff => "TIFF",
				F::Tga => "TGA",
				F::Dds => "DDS",
				F::Bmp => "BMP",
				F::Ico => "ICO",
				F::Hdr => "HDR",
				F::OpenExr => "OpenEXR",
				F::Farbfeld => "Farbfeld",
				F::Avif => "AVIF",
				F::Qoi => "QOI",
				_ => "Unknown",
			})
		});
	}
}

// --- ImageColor
struct ImageColor(image::ColorType);

impl UserData for ImageColor {
	fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
		methods.add_meta_method(MetaMethod::ToString, |_, me, ()| {
			use image::ColorType as C;

			Ok(match me.0 {
				C::L8 => "L8",
				C::La8 => "La8",
				C::Rgb8 => "Rgb8",
				C::Rgba8 => "Rgba8",

				C::L16 => "L16",
				C::La16 => "La16",
				C::Rgb16 => "Rgb16",
				C::Rgba16 => "Rgba16",

				C::Rgb32F => "Rgb32F",
				C::Rgba32F => "Rgba32F",
				_ => "Unknown",
			})
		});
	}
}

#[cfg(test)]
mod tests {
	use std::{path::PathBuf, sync::atomic::{AtomicU64, Ordering}};

	use image::{DynamicImage, ImageFormat};

	use super::ImageInfo;

	static SEQ: AtomicU64 = AtomicU64::new(0);

	// Unique scratch path without extra dependencies or process-env use.
	fn scratch(suffix: &str) -> PathBuf {
		let n = SEQ.fetch_add(1, Ordering::Relaxed);
		std::env::temp_dir().join(format!("yazi-image-info-{}-{n}.{suffix}", std::process::id()))
	}

	#[tokio::test]
	async fn info_reports_format_dimensions_and_color() {
		let img = DynamicImage::new_rgb8(7, 5);
		let mut buf = vec![];
		let mut cursor = std::io::Cursor::new(&mut buf);
		img.write_to(&mut cursor, ImageFormat::Png).unwrap();
		drop(cursor);

		let path = scratch("png");
		tokio::fs::write(&path, &buf).await.unwrap();
		let info = ImageInfo::new(path.clone()).await.unwrap();
		tokio::fs::remove_file(&path).await.ok();

		// The fields the Lua metadata preview renders.
		assert!(matches!(info.format, ImageFormat::Png));
		assert_eq!((info.width, info.height), (7, 5));
		assert!(matches!(info.color, image::ColorType::Rgb8));
	}

	#[tokio::test]
	async fn undecodable_image_keeps_a_real_error() {
		let path = scratch("bin");
		tokio::fs::write(&path, b"this is not an image").await.unwrap();
		let err = match ImageInfo::new(path.clone()).await {
			Ok(_) => panic!("malformed image must not decode"),
			Err(e) => e,
		};
		tokio::fs::remove_file(&path).await.ok();

		assert!(!err.to_string().is_empty());
	}
}
