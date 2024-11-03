use adw::prelude::FileExt;
use gio::{Cancellable, FileQueryInfoFlags};
use gtk::gio;
use image::*;
use log::*;
use resvg::tiny_skia::Pixmap;
use resvg::usvg::{Options, Tree};
use std::error::Error;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub files: Option<gio::File>,
    pub path: PathBuf,
    pub filename: String,
    pub extension: String,
    pub dynamic_image: DynamicImage,
    pub thumbnail: DynamicImage,
    pub hash: u64,
}

impl File {
    pub fn path_str(&self) -> String {
        self.path.clone().into_os_string().into_string().unwrap()
    }
    pub fn new(file: gio::File, size: i32, thumbnail_size: i32) -> Result<Self, Box<dyn Error>> {
        let temp_path = file.path().unwrap();
        let file_info =
            file.query_info("standard::", FileQueryInfoFlags::NONE, Cancellable::NONE)?;
        let file_name = file_info.name().into_os_string().into_string().unwrap();
        let period_split: Vec<&str> = file_name.split(".").collect();
        let file_extension = format!(".{}", period_split.last().unwrap());
        let mime_type = file_info.content_type();
        debug!("Mime type: {:?}", mime_type);
        let dynamic_image = if mime_type == Some("image/svg+xml".into()) {
            let path = &temp_path.as_os_str().to_str().unwrap();
            Self::load_svg(path, size)?
        } else {
            image::open(temp_path.clone().into_os_string())?
        };
        let name_no_extension = file_name.replace(&file_extension, "");
        let hash = Self::create_hash(&dynamic_image);
        let thumbnail = if file_extension == ".svg" {
            let path = &temp_path.as_os_str().to_str().unwrap();
            Self::load_svg(path, thumbnail_size)?
        } else {
            dynamic_image.clone().resize(
                thumbnail_size as u32,
                thumbnail_size as u32,
                imageops::FilterType::Nearest,
            )
        };
        Ok(Self {
            files: Some(file),
            path: temp_path.into(),
            extension: mime_type.unwrap().into(),
            filename: name_no_extension,
            dynamic_image,
            thumbnail,
            hash,
        })
    }

    pub fn from_path_string(
        path: &str,
        size: i32,
        thumbnail_size: i32,
    ) -> Result<Self, Box<dyn Error>> {
        //let thumbnail = file.clone().resize(255, 255, imageops::FilterType::Nearest);
        let file = gio::File::for_path(PathBuf::from(path).as_path());
        Self::new(file, size, thumbnail_size)
    }

    pub fn from_path(
        path: PathBuf,
        size: i32,
        thumbnail_size: i32,
    ) -> Result<Self, Box<dyn Error>> {
        //let thumbnail = file.clone().resize(255, 255, imageops::FilterType::Nearest);
        let file = gio::File::for_path(path);
        Self::new(file, size, thumbnail_size)
    }

    pub fn from_image(image: DynamicImage, thumbnail_size: i32, filename: &str) -> Self {
        let thumbnail = image.clone().resize(
            thumbnail_size as u32,
            thumbnail_size as u32,
            imageops::FilterType::Nearest,
        );
        let hash = Self::create_hash(&image);
        Self {
            files: None,
            path: "".into(),
            extension: ".dynamic".to_string(),
            filename: filename.to_string(),
            hash,
            dynamic_image: image,
            thumbnail,
        }
    }

    pub fn load_svg(path: &str, size: i32) -> Result<DynamicImage, Box<dyn Error>> {
        // Load the SVG file content
        let svg_data = match fs::read(path) {
            Ok(x) => x,
            Err(_) => fs::read("/usr/share/icons/Adwaita/scalable/places/folder.svg")?,
        };
        // Create an SVG tree
        let opt = Options::default();
        let rtree = Tree::from_data(&svg_data, &opt)?;

        // Specify the output dimensions (you can adjust these as needed)
        let width = rtree.size().width();
        let height = rtree.size().height();

        // Calculate the scale factor
        let scale_x = size as f32 / width;
        let scale_y = size as f32 / height;
        let scale = scale_x.min(scale_y); // Maintain aspect ratio

        // Create a Pixmap to render into
        let mut pixmap = Pixmap::new(size as u32, size as u32).unwrap();

        // Render the SVG tree to the Pixmap
        let _ = resvg::render(
            &rtree,
            usvg::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        Ok(Self::pixmap_to_image(pixmap))
    }

    // Adds hash of file to file name
    fn create_hash(image: &DynamicImage) -> u64 {
        let mut hasher = DefaultHasher::new();
        let _ = image.as_rgb8().hash(&mut hasher);
        hasher.finish()
    }

    fn pixmap_to_image(pixmap: Pixmap) -> DynamicImage {
        // Create an empty RgbaImage with the same dimensions as the Pixmap.
        let pixmap_clone = pixmap.clone();
        let mut img = RgbaImage::new(pixmap_clone.width(), pixmap.height());

        // Iterate over each pixel in the Pixmap.
        for y in 0..pixmap_clone.height() {
            for x in 0..pixmap_clone.width() {
                // Get the pixel at (x, y). `pixel` returns an Option, we unwrap it safely because we know (x, y) is valid.
                if let Some(pixel) = pixmap_clone.pixel(x, y) {
                    // Copy the pixel's RGBA data into the RgbaImage.
                    img.put_pixel(
                        x,
                        y,
                        Rgba([pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]),
                    );
                }
            }
        }

        // Convert the RgbaImage to a DynamicImage.
        DynamicImage::ImageRgba8(img)
    }
}
