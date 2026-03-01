#![cfg_attr(windows, feature(abi_vectorcall))]

use std::sync::Mutex;

use ext_php_rs::prelude::*;

static VISITORS : Mutex<u32> = Mutex::new(0);

#[php_function]
fn get_visitors() -> u32 {
    let mut v = VISITORS.lock().unwrap();
    *v+=1;
    *v
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(get_visitors))
}
