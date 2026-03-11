#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::{exception::PhpException, prelude::*, zend::ce};


mod image;
mod image_decode;
mod image_encode;
mod image_error;
mod image_info;
mod image_ops;

// ── Module Registration ─────────────────────────────────────────────

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .class::<image_error::ImageException>()
        .class::<image_info::ImageInfo>()
        .class::<image::PhpImage>()
}
