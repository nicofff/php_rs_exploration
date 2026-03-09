use ext_php_rs::{exception::PhpException, prelude::*, zend::ce};

#[php_class]
#[php(name = "RustImage\\ImageException")]
#[php(extends(ce = ce::exception, stub = "\\Exception"))]
#[derive(Default)]
pub struct ImageException;

pub struct ImageError(pub String);

impl From<String> for ImageError {
    fn from(msg: String) -> Self {
        Self(msg)
    }
}

impl From<&str> for ImageError {
    fn from(msg: &str) -> Self {
        Self(msg.to_string())
    }
}

impl From<image::ImageError> for ImageError {
    fn from(err: image::ImageError) -> Self {
        Self(err.to_string())
    }
}

impl From<std::io::Error> for ImageError {
    fn from(err: std::io::Error) -> Self {
        Self(err.to_string())
    }
}

impl From<ImageError> for PhpException {
    fn from(err: ImageError) -> Self {
        PhpException::from_class::<ImageException>(err.0)
    }
}
