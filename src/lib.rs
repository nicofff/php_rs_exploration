#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::{exception::PhpException, prelude::*, zend::ce};
use redis::RedisError;

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
}

// ── Module Registration ─────────────────────────────────────────────

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .class::<PhpRedisException>()
        .class::<RedisClient>()
}
