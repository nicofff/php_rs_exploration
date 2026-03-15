#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::prelude::*;

mod image;
mod image_error;
mod image_info;
mod rgb;

// ── Module Registration ─────────────────────────────────────────────

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .class::<image_error::ImageException>()
        .class::<image_info::ImageInfo>()
        .class::<image::PhpImage>()
        .class::<rgb::PhpRgb>()
}
