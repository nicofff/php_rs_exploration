use std::collections::HashMap;
use ext_php_rs::prelude::*;

#[php_class]
#[php(name = "RustImage\\ImageInfo")]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub has_alpha: bool,
    pub is_animated: bool,
    pub exif_data: Option<HashMap<String, String>>,
}

#[php_impl]
impl ImageInfo {
    #[php(getter)]
    pub fn width(&self) -> i64 {
        self.width as i64
    }

    #[php(getter)]
    pub fn height(&self) -> i64 {
        self.height as i64
    }

    #[php(getter)]
    pub fn format(&self) -> String {
        self.format.clone()
    }

    #[php(getter)]
    pub fn has_alpha(&self) -> bool {
        self.has_alpha
    }

    #[php(getter)]
    pub fn is_animated(&self) -> bool {
        self.is_animated
    }

    #[php(getter)]
    pub fn exif(&self) -> Option<HashMap<String, String>> {
        self.exif_data.clone()
    }
}
