#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::{exception::PhpException, prelude::*, zend::ce};
use redis::{Commands, RedisError};
use std::collections::HashMap;

// ── Error Handling ──────────────────────────────────────────────────

#[php_class]
#[php(name = "Redis\\RedisException")]
#[php(extends(ce = ce::exception, stub = "\\Exception"))]
#[derive(Default)]
pub struct PhpRedisException;

struct RedisErrorWrapper(RedisError);

impl From<RedisError> for RedisErrorWrapper {
    fn from(value: RedisError) -> Self {
        Self(value)
    }
}

impl From<RedisErrorWrapper> for PhpException {
    fn from(value: RedisErrorWrapper) -> Self {
        PhpException::from_class::<PhpRedisException>(value.0.to_string())
    }
}

// ── Redis Client ────────────────────────────────────────────────────

#[php_class]
#[php(name = "Redis\\Client")]
struct RedisClient {
    connection: redis::Connection,
}

#[php_impl]
impl RedisClient {
    // ── Connection ──────────────────────────────────────────────

    fn __construct(dsn: &str) -> Result<Self, RedisErrorWrapper> {
        let client = redis::Client::open(dsn)?;
        let connection = client.get_connection()?;
        Ok(Self { connection })
    }

    fn ping(&mut self) -> Result<String, RedisErrorWrapper> {
        Ok(redis::cmd("PING").query(&mut self.connection)?)
    }

    // ── String Commands ─────────────────────────────────────────

    fn get(&mut self, key: &str) -> Result<Option<String>, RedisErrorWrapper> {
        Ok(self.connection.get(key)?)
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), RedisErrorWrapper> {
        Ok(self.connection.set(key, value)?)
    }

    fn set_ex(&mut self, key: &str, value: &str, seconds: u64) -> Result<(), RedisErrorWrapper> {
        Ok(self.connection.set_ex(key, value, seconds)?)
    }

    fn set_nx(&mut self, key: &str, value: &str) -> Result<bool, RedisErrorWrapper> {
        Ok(self.connection.set_nx(key, value)?)
    }

    fn mget(&mut self, keys: Vec<String>) -> Result<Vec<Option<String>>, RedisErrorWrapper> {
        Ok(self.connection.mget(keys)?)
    }

    fn mset(&mut self, items: HashMap<String, String>) -> Result<(), RedisErrorWrapper> {
        let pairs: Vec<(String, String)> = items.into_iter().collect();
        Ok(self.connection.mset(&pairs)?)
    }

    fn incr(&mut self, key: &str, delta: i64) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.incr(key, delta)?)
    }

    fn decr(&mut self, key: &str, delta: i64) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.decr(key, delta)?)
    }

    fn append(&mut self, key: &str, value: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.append(key, value)?)
    }

    fn strlen(&mut self, key: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.strlen(key)?)
    }

    fn getset(&mut self, key: &str, value: &str) -> Result<Option<String>, RedisErrorWrapper> {
        Ok(self.connection.getset(key, value)?)
    }

    fn getrange(&mut self, key: &str, from: isize, to: isize) -> Result<String, RedisErrorWrapper> {
        Ok(self.connection.getrange(key, from, to)?)
    }

    fn setrange(&mut self, key: &str, offset: isize, value: &str) -> Result<i64, RedisErrorWrapper> {
        let result: usize = self.connection.setrange(key, offset, value)?;
        Ok(result as i64)
    }

    fn get_del(&mut self, key: &str) -> Result<Option<String>, RedisErrorWrapper> {
        Ok(self.connection.get_del(key)?)
    }
}

// ── Module Registration ─────────────────────────────────────────────

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .class::<PhpRedisException>()
        .class::<RedisClient>()
}
