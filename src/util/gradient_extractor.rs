use gtk::gdk::prelude::TextureExt;
use image::RgbaImage;

use crate::util::color::RGBAColor;

pub struct GradientColorExtractor;

impl GradientColorExtractor {
    pub fn extract_from_raw_bytes(
        bytes: &[u8],
        width: u32,
        height: u32,
        color_count: u32,
    ) -> Vec<RGBAColor> {
        if bytes.is_empty() || color_count == 0 || width == 0 || height == 0 {
            return Vec::new();
        }

        let Some(img) = RgbaImage::from_raw(width, height, bytes.to_vec()) else {
            return Vec::new();
        };

        let tile_height = height / color_count;
        let mut extracted_colors = Vec::new();

        for i in 0..color_count {
            let start_y = i * tile_height;
            let end_y = if i == color_count - 1 {
                height
            } else {
                start_y + tile_height
            };

            let mut total_r: u64 = 0;
            let mut total_g: u64 = 0;
            let mut total_b: u64 = 0;
            let mut total_a: u64 = 0;
            let mut pixel_count: u64 = 0;

            for y in start_y..end_y {
                for x in 0..width {
                    let pixel = img.get_pixel(x, y);
                    total_r += pixel[0] as u64;
                    total_g += pixel[1] as u64;
                    total_b += pixel[2] as u64;
                    total_a += pixel[3] as u64;
                    pixel_count += 1;
                }
            }

            if pixel_count > 0 {
                extracted_colors.push(RGBAColor {
                    red: (total_r as f64 / pixel_count as f64) / 255.0,
                    green: (total_g as f64 / pixel_count as f64) / 255.0,
                    blue: (total_b as f64 / pixel_count as f64) / 255.0,
                    alpha: (total_a as f64 / pixel_count as f64) / 255.0,
                });
            }
        }

        extracted_colors
    }

    fn gradient_css(gradient_colors: Vec<RGBAColor>) -> String {
        if gradient_colors.is_empty() {
            return "background: rgba(0,0,0,1);".to_string();
        }
        let mut stops = Vec::new();
        let len = gradient_colors.len();

        if len == 1 {
            let color = gradient_colors[0];
            let rgba_str = format!(
                "rgba({}, {}, {}, {})",
                (color.red * 255.0) as u8,
                (color.green * 255.0) as u8,
                (color.blue * 255.0) as u8,
                color.alpha
            );
            stops.push(format!("{} 0%", rgba_str));
            stops.push(format!("{} 100%", rgba_str));
        } else {
            for (i, color) in gradient_colors.iter().rev().enumerate() {
                let percentage = (i as f64 / (len - 1) as f64) * 100.0;
                stops.push(format!(
                    "rgba({}, {}, {}, {}) {:.1}%",
                    (color.red * 255.0) as u8,
                    (color.green * 255.0) as u8,
                    (color.blue * 255.0) as u8,
                    color.alpha,
                    percentage
                ));
            }
        }
        let linear_grad = format!("linear-gradient(135deg, {})", stops.join(", "));
        format!(
            "background-image: linear-gradient(rgba(0,0,0,0.45), rgba(0,0,0,0.45)), {};",
            linear_grad
        )
    }

    fn get_raw_rgba_bytes(texture: &gtk::gdk::Texture) -> Vec<u8> {
        let mut downloader = gtk::gdk::TextureDownloader::new(texture);

        // Explicitly set the format to standard, non-premultiplied RGBA
        downloader.set_format(gtk::gdk::MemoryFormat::R8g8b8a8);

        // Downloads and converts the data perfectly to straight R8G8B8A8 bytes
        let (bytes, _stride) = downloader.download_bytes();

        bytes.to_vec()
    }

    pub fn extract_css_gradient_from_texture(texture: &gtk::gdk::Texture) -> String {
        let width = texture.width() as u32;
        let height = texture.height() as u32;
        let bytes = Self::get_raw_rgba_bytes(texture);

        Self::gradient_css(Self::extract_from_raw_bytes(&bytes, width, height, 3))
    }
}
