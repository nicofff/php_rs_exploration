#![cfg_attr(windows, feature(abi_vectorcall))]

use std::{thread::{self, sleep}, time::Duration};

use ext_php_rs::prelude::*;

#[php_function]
fn say_hello_from_thread(name: String) {
    thread::spawn(move || {
        for x in 0..5 {
            php_println!("{} Hello, {}!", x,  name);
            sleep(Duration::from_secs(1));
        }
    });
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(say_hello_from_thread))
}
