#![cfg_attr(windows, feature(abi_vectorcall))]

use std::{thread::{self, sleep}, time::Duration};

use ext_php_rs::prelude::*;
use redis::{RedisError, TypedCommands as _};

struct RedisException {
    inner: RedisError
}

// impl Into<PhpException> for RedisException {
//     fn into(self) -> PhpException {
//         PhpException::default(self.inner.detail().unwrap().to_string())
//     }
// }

impl From<RedisException> for PhpException {
    fn from(value: RedisException) -> Self {
        //eprintln!("{:?}", value.inner.kind());
        PhpException::default(value.inner.to_string())
    }
}

impl From<RedisError> for RedisException {
    fn from(value: RedisError) -> Self {
        RedisException { inner: value }
    }
}


#[php_class]
struct RedisWrapper {
    client: redis::Client,
}


#[php_impl]
impl RedisWrapper {
    fn __construct(server: &str) -> Result<Self, RedisException> {
        let client = redis::Client::open(server)?;
        Ok(Self { client })
    }
    
    fn get_key(&mut self, key: &str) -> Result<Option<String>, RedisException> {
        Ok(self.client.get(key)?)
    }
}


#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.class::<RedisWrapper>()
}
