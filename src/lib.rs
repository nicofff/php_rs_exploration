#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::{exception::PhpException, prelude::*, zend::ce};
use redis::{Commands, RedisError};
use std::collections::{HashMap, HashSet};

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

    // ── Key Commands ────────────────────────────────────────────

    fn del(&mut self, key: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.del(key)?)
    }

    fn exists(&mut self, key: &str) -> Result<bool, RedisErrorWrapper> {
        Ok(self.connection.exists(key)?)
    }

    fn expire(&mut self, key: &str, seconds: i64) -> Result<bool, RedisErrorWrapper> {
        Ok(self.connection.expire(key, seconds)?)
    }

    fn expire_at(&mut self, key: &str, ts: i64) -> Result<bool, RedisErrorWrapper> {
        Ok(self.connection.expire_at(key, ts)?)
    }

    fn persist(&mut self, key: &str) -> Result<bool, RedisErrorWrapper> {
        Ok(self.connection.persist(key)?)
    }

    fn keys(&mut self, pattern: &str) -> Result<Vec<String>, RedisErrorWrapper> {
        Ok(self.connection.keys(pattern)?)
    }

    fn rename(&mut self, key: &str, new_key: &str) -> Result<(), RedisErrorWrapper> {
        Ok(self.connection.rename(key, new_key)?)
    }

    fn rename_nx(&mut self, key: &str, new_key: &str) -> Result<bool, RedisErrorWrapper> {
        Ok(self.connection.rename_nx(key, new_key)?)
    }

    fn unlink(&mut self, key: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.unlink(key)?)
    }

    // ── Hash Commands ───────────────────────────────────────────

    fn hget(&mut self, key: &str, field: &str) -> Result<Option<String>, RedisErrorWrapper> {
        Ok(self.connection.hget(key, field)?)
    }

    fn hset(&mut self, key: &str, field: &str, value: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.hset(key, field, value)?)
    }

    fn hdel(&mut self, key: &str, field: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.hdel(key, field)?)
    }

    fn hgetall(&mut self, key: &str) -> Result<HashMap<String, String>, RedisErrorWrapper> {
        Ok(self.connection.hgetall(key)?)
    }

    fn hkeys(&mut self, key: &str) -> Result<Vec<String>, RedisErrorWrapper> {
        Ok(self.connection.hkeys(key)?)
    }

    fn hvals(&mut self, key: &str) -> Result<Vec<String>, RedisErrorWrapper> {
        Ok(self.connection.hvals(key)?)
    }

    fn hexists(&mut self, key: &str, field: &str) -> Result<bool, RedisErrorWrapper> {
        Ok(self.connection.hexists(key, field)?)
    }

    fn hlen(&mut self, key: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.hlen(key)?)
    }

    fn hset_nx(&mut self, key: &str, field: &str, value: &str) -> Result<bool, RedisErrorWrapper> {
        Ok(self.connection.hset_nx(key, field, value)?)
    }

    fn hincr(&mut self, key: &str, field: &str, delta: f64) -> Result<f64, RedisErrorWrapper> {
        Ok(self.connection.hincr(key, field, delta)?)
    }

    fn hmget(&mut self, key: &str, fields: Vec<String>) -> Result<Vec<String>, RedisErrorWrapper> {
        let mut cmd = redis::cmd("HMGET");
        cmd.arg(key);
        for f in &fields {
            cmd.arg(f);
        }
        Ok(cmd.query(&mut self.connection)?)
    }

    fn hset_multiple(&mut self, key: &str, items: HashMap<String, String>) -> Result<(), RedisErrorWrapper> {
        let pairs: Vec<(String, String)> = items.into_iter().collect();
        Ok(self.connection.hset_multiple(key, &pairs)?)
    }

    // ── List Commands ───────────────────────────────────────────

    fn lpush(&mut self, key: &str, value: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.lpush(key, value)?)
    }

    fn rpush(&mut self, key: &str, value: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.rpush(key, value)?)
    }

    fn lpop(&mut self, key: &str) -> Result<Option<String>, RedisErrorWrapper> {
        Ok(redis::cmd("LPOP").arg(key).query(&mut self.connection)?)
    }

    fn rpop(&mut self, key: &str) -> Result<Option<String>, RedisErrorWrapper> {
        Ok(redis::cmd("RPOP").arg(key).query(&mut self.connection)?)
    }

    fn llen(&mut self, key: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.llen(key)?)
    }

    fn lrange(&mut self, key: &str, start: isize, stop: isize) -> Result<Vec<String>, RedisErrorWrapper> {
        Ok(self.connection.lrange(key, start, stop)?)
    }

    fn lindex(&mut self, key: &str, index: isize) -> Result<Option<String>, RedisErrorWrapper> {
        Ok(self.connection.lindex(key, index)?)
    }

    fn lset(&mut self, key: &str, index: isize, value: &str) -> Result<(), RedisErrorWrapper> {
        Ok(self.connection.lset(key, index, value)?)
    }

    fn lrem(&mut self, key: &str, count: isize, value: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.lrem(key, count, value)?)
    }

    fn ltrim(&mut self, key: &str, start: isize, stop: isize) -> Result<(), RedisErrorWrapper> {
        Ok(self.connection.ltrim(key, start, stop)?)
    }

    fn linsert_before(&mut self, key: &str, pivot: &str, value: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.linsert_before(key, pivot, value)?)
    }

    fn linsert_after(&mut self, key: &str, pivot: &str, value: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.linsert_after(key, pivot, value)?)
    }

    fn lpush_exists(&mut self, key: &str, value: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.lpush_exists(key, value)?)
    }

    fn rpush_exists(&mut self, key: &str, value: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.rpush_exists(key, value)?)
    }

    // ── Set Commands ────────────────────────────────────────────

    fn sadd(&mut self, key: &str, member: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.sadd(key, member)?)
    }

    fn srem(&mut self, key: &str, member: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.srem(key, member)?)
    }

    fn smembers(&mut self, key: &str) -> Result<HashSet<String>, RedisErrorWrapper> {
        Ok(self.connection.smembers(key)?)
    }

    fn sismember(&mut self, key: &str, member: &str) -> Result<bool, RedisErrorWrapper> {
        Ok(self.connection.sismember(key, member)?)
    }

    fn scard(&mut self, key: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.scard(key)?)
    }

    fn sdiff(&mut self, keys: Vec<String>) -> Result<HashSet<String>, RedisErrorWrapper> {
        Ok(self.connection.sdiff(keys)?)
    }

    fn sinter(&mut self, keys: Vec<String>) -> Result<HashSet<String>, RedisErrorWrapper> {
        Ok(self.connection.sinter(keys)?)
    }

    fn sunion(&mut self, keys: Vec<String>) -> Result<HashSet<String>, RedisErrorWrapper> {
        Ok(self.connection.sunion(keys)?)
    }

    fn smove(&mut self, src: &str, dst: &str, member: &str) -> Result<bool, RedisErrorWrapper> {
        Ok(self.connection.smove(src, dst, member)?)
    }

    fn spop(&mut self, key: &str) -> Result<Option<String>, RedisErrorWrapper> {
        Ok(redis::cmd("SPOP").arg(key).query(&mut self.connection)?)
    }

    fn srandmember(&mut self, key: &str) -> Result<Option<String>, RedisErrorWrapper> {
        Ok(self.connection.srandmember(key)?)
    }

    // ── Sorted Set Commands ─────────────────────────────────────

    fn zadd(&mut self, key: &str, member: &str, score: f64) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.zadd(key, member, score)?)
    }

    fn zrem(&mut self, key: &str, member: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.zrem(key, member)?)
    }

    fn zrange(&mut self, key: &str, start: isize, stop: isize) -> Result<Vec<String>, RedisErrorWrapper> {
        Ok(self.connection.zrange(key, start, stop)?)
    }

    fn zrevrange(&mut self, key: &str, start: isize, stop: isize) -> Result<Vec<String>, RedisErrorWrapper> {
        Ok(self.connection.zrevrange(key, start, stop)?)
    }

    fn zrank(&mut self, key: &str, member: &str) -> Result<Option<i64>, RedisErrorWrapper> {
        Ok(self.connection.zrank(key, member)?)
    }

    fn zrevrank(&mut self, key: &str, member: &str) -> Result<Option<i64>, RedisErrorWrapper> {
        Ok(self.connection.zrevrank(key, member)?)
    }

    fn zscore(&mut self, key: &str, member: &str) -> Result<Option<f64>, RedisErrorWrapper> {
        Ok(self.connection.zscore(key, member)?)
    }

    fn zcard(&mut self, key: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.zcard(key)?)
    }

    fn zcount(&mut self, key: &str, min: &str, max: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.zcount(key, min, max)?)
    }

    fn zincr(&mut self, key: &str, member: &str, delta: f64) -> Result<f64, RedisErrorWrapper> {
        Ok(self.connection.zincr(key, member, delta)?)
    }

    fn zrangebyscore(&mut self, key: &str, min: &str, max: &str) -> Result<Vec<String>, RedisErrorWrapper> {
        Ok(self.connection.zrangebyscore(key, min, max)?)
    }

    // ── Pub/Sub ─────────────────────────────────────────────────

    fn publish(&mut self, channel: &str, message: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.publish(channel, message)?)
    }

    // ── Server Commands ─────────────────────────────────────────

    fn flushdb(&mut self) -> Result<(), RedisErrorWrapper> {
        Ok(self.connection.flushdb()?)
    }

    fn flushall(&mut self) -> Result<(), RedisErrorWrapper> {
        Ok(self.connection.flushall()?)
    }
}

// ── Module Registration ─────────────────────────────────────────────

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .class::<PhpRedisException>()
        .class::<RedisClient>()
}
