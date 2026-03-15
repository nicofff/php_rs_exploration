use ril::Rgba;
use ext_php_rs::prelude::*;

#[derive(Default)]
#[php_class]
#[php(name = "RustImage\\Rgb")]
pub struct PhpRgb {
    r: u8,
    g: u8,
    b: u8,
}

#[php_impl]
impl PhpRgb {
    pub fn __construct(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    #[php(getter)]
    pub fn r(&self) -> u8 { self.r }
    #[php(getter)]
    pub fn g(&self) -> u8 { self.g }
    #[php(getter)]
    pub fn b(&self) -> u8 { self.b }
}

impl PhpRgb {
    pub(crate) fn to_rgba(&self) -> Rgba {
        Rgba { r: self.r, g: self.g, b: self.b, a: 255 }
    }
}
