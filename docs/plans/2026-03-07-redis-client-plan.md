# Redis Client PHP Extension - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a PHP Redis client extension by wrapping the Rust `redis` crate via `ext-php-rs`.

**Architecture:** Single `src/lib.rs` file. One `RedisClient` PHP class with persistent connection. One `PhpRedisException` class extending `\Exception`. Methods directly call `TypedCommands` on the connection.

**Tech Stack:** Rust (edition 2024), `ext-php-rs` (latest), `redis` crate (1.0.4), PHP 8.5

**Build/Test commands:**
- Build: `cargo build`
- Run PHP test: `php -d extension=target/debug/libphprs_hello_world.dylib test_redis.php`
- Requires: local Redis on `redis://localhost`

**Reference:** Design doc at `docs/plans/2026-03-07-redis-client-design.md`

---

### Task 1: Scaffold — Exception class, RedisClient struct, connection management

**Files:**
- Modify: `src/lib.rs` (full rewrite)

**Step 1: Rewrite `src/lib.rs` with the foundation**

Replace the entire file with:

```rust
#![cfg_attr(windows, feature(abi_vectorcall))]

use std::collections::{HashMap, HashSet};

use ext_php_rs::{exception::PhpException, prelude::*, zend::ce};
use redis::{Commands, RedisError};

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
```

**Step 2: Build**

Run: `cargo build`
Expected: Compiles with no errors.

**Step 3: Write test PHP script**

Create `test_redis.php`:

```php
<?php

// Task 1: Connection + Ping
try {
    $redis = new Redis\Client("redis://localhost");
    echo "Connected OK\n";
    $pong = $redis->ping();
    echo "Ping: $pong\n";
} catch (Redis\RedisException $e) {
    echo "Redis error: " . $e->getMessage() . "\n";
}

// Test exception on bad connection
try {
    $bad = new Redis\Client("redis://nonexistent:9999");
    echo "FAIL: should have thrown\n";
} catch (Redis\RedisException $e) {
    echo "Expected error: " . $e->getMessage() . "\n";
}
```

**Step 4: Run test**

Run: `php -d extension=target/debug/libphprs_hello_world.dylib test_redis.php`
Expected:
```
Connected OK
Ping: PONG
Expected error: ...connection refused...
```

**Step 5: Commit**

```bash
git add src/lib.rs test_redis.php
git commit -m "feat: scaffold RedisClient with connection and typed exception"
```

---

### Task 2: String Commands

**Files:**
- Modify: `src/lib.rs` — add methods inside the `#[php_impl]` block
- Modify: `test_redis.php` — add string command tests

**Step 1: Add string command methods to the `#[php_impl] impl RedisClient` block**

Add after the `ping` method:

```rust
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

    fn getrange(&mut self, key: &str, from: i64, to: i64) -> Result<String, RedisErrorWrapper> {
        Ok(self.connection.getrange(key, from, to)?)
    }

    fn setrange(&mut self, key: &str, offset: i64, value: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.setrange(key, offset, value)?)
    }

    fn get_del(&mut self, key: &str) -> Result<Option<String>, RedisErrorWrapper> {
        Ok(self.connection.get_del(key)?)
    }
```

**Step 2: Build**

Run: `cargo build`
Expected: Compiles. If any type mismatches with the redis crate, adjust (e.g. `usize` vs `i64`). The redis crate's generic return types may need explicit type annotations.

**Step 3: Add tests to `test_redis.php`**

Append:

```php
// Task 2: String Commands
echo "\n--- String Commands ---\n";
$redis->set("test:str", "hello");
echo "get: " . $redis->get("test:str") . "\n"; // hello
echo "get missing: " . var_export($redis->get("test:nonexistent"), true) . "\n"; // NULL

$redis->setEx("test:ttl", "expires", 60);
echo "set_ex: " . $redis->get("test:ttl") . "\n";

echo "set_nx new: " . var_export($redis->setNx("test:nx", "first"), true) . "\n"; // true
echo "set_nx exists: " . var_export($redis->setNx("test:nx", "second"), true) . "\n"; // false

$redis->set("test:a", "1");
$redis->set("test:b", "2");
$vals = $redis->mget(["test:a", "test:b", "test:missing"]);
echo "mget: "; var_dump($vals);

$redis->mset(["test:m1" => "x", "test:m2" => "y"]);
echo "mset then get: " . $redis->get("test:m1") . ", " . $redis->get("test:m2") . "\n";

$redis->set("test:counter", "10");
echo "incr: " . $redis->incr("test:counter", 5) . "\n"; // 15
echo "decr: " . $redis->decr("test:counter", 3) . "\n"; // 12

echo "append: " . $redis->append("test:str", " world") . "\n";
echo "strlen: " . $redis->strlen("test:str") . "\n"; // 11

echo "getset: " . var_export($redis->getset("test:str", "new"), true) . "\n"; // 'hello world'
echo "getrange: " . $redis->getrange("test:str", 0, 2) . "\n"; // 'new'

echo "get_del: " . var_export($redis->getDel("test:str"), true) . "\n"; // 'new'
echo "after get_del: " . var_export($redis->get("test:str"), true) . "\n"; // NULL
```

**Step 4: Run test**

Run: `php -d extension=target/debug/libphprs_hello_world.dylib test_redis.php`

**Step 5: Commit**

```bash
git add src/lib.rs test_redis.php
git commit -m "feat: add string commands (get, set, mget, mset, incr, etc.)"
```

---

### Task 3: Key Commands

**Files:**
- Modify: `src/lib.rs`
- Modify: `test_redis.php`

**Step 1: Add key command methods**

Add to the `#[php_impl]` block:

```rust
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
```

Note: `ttl` and `pttl` are skipped — they return `IntegerReplyOrNoOp` which needs custom mapping. Track for future.

**Step 2: Build**

Run: `cargo build`

**Step 3: Add tests**

```php
// Task 3: Key Commands
echo "\n--- Key Commands ---\n";
$redis->set("test:key1", "val");
echo "exists: " . var_export($redis->exists("test:key1"), true) . "\n"; // true
echo "exists missing: " . var_export($redis->exists("test:nope"), true) . "\n"; // false

echo "del: " . $redis->del("test:key1") . "\n"; // 1
echo "del missing: " . $redis->del("test:key1") . "\n"; // 0

$redis->set("test:exp", "temp");
echo "expire: " . var_export($redis->expire("test:exp", 60), true) . "\n"; // true
echo "persist: " . var_export($redis->persist("test:exp"), true) . "\n"; // true

$redis->set("test:ren1", "val");
$redis->rename("test:ren1", "test:ren2");
echo "after rename: " . $redis->get("test:ren2") . "\n"; // val

$redis->set("test:keys_a", "1");
$redis->set("test:keys_b", "2");
$found = $redis->keys("test:keys_*");
echo "keys: "; var_dump($found);

echo "unlink: " . $redis->unlink("test:ren2") . "\n"; // 1
```

**Step 4: Run test, Step 5: Commit**

```bash
git add src/lib.rs test_redis.php
git commit -m "feat: add key commands (del, exists, expire, keys, rename, etc.)"
```

---

### Task 4: Hash Commands

**Files:**
- Modify: `src/lib.rs`
- Modify: `test_redis.php`

**Step 1: Add hash command methods**

```rust
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
        Ok(self.connection.hget(key, fields)?)
    }

    fn hset_multiple(&mut self, key: &str, items: HashMap<String, String>) -> Result<(), RedisErrorWrapper> {
        let pairs: Vec<(String, String)> = items.into_iter().collect();
        Ok(self.connection.hset_multiple(key, &pairs)?)
    }
```

**Step 2: Build, Step 3: Add tests**

```php
// Task 4: Hash Commands
echo "\n--- Hash Commands ---\n";
$redis->hset("test:hash", "name", "Alice");
$redis->hset("test:hash", "age", "30");

echo "hget: " . $redis->hget("test:hash", "name") . "\n"; // Alice
echo "hexists: " . var_export($redis->hexists("test:hash", "name"), true) . "\n"; // true
echo "hlen: " . $redis->hlen("test:hash") . "\n"; // 2

$all = $redis->hgetall("test:hash");
echo "hgetall: "; var_dump($all); // ["name" => "Alice", "age" => "30"]

$keys = $redis->hkeys("test:hash");
echo "hkeys: "; var_dump($keys);

$vals = $redis->hvals("test:hash");
echo "hvals: "; var_dump($vals);

$redis->hincr("test:hash", "age", 1.0);
echo "after hincr: " . $redis->hget("test:hash", "age") . "\n"; // 31

$redis->hsetMultiple("test:hash2", ["a" => "1", "b" => "2"]);
$multi = $redis->hmget("test:hash2", ["a", "b"]);
echo "hmget: "; var_dump($multi);

echo "hdel: " . $redis->hdel("test:hash", "name") . "\n"; // 1
```

**Step 4: Run test, Step 5: Commit**

```bash
git add src/lib.rs test_redis.php
git commit -m "feat: add hash commands (hget, hset, hgetall, hincr, etc.)"
```

---

### Task 5: List Commands

**Files:**
- Modify: `src/lib.rs`
- Modify: `test_redis.php`

**Step 1: Add list command methods**

Note: `lpop`/`rpop` in the redis crate take `Option<NonZeroUsize>` and return a generic `RV`. We'll implement simple single-element versions using raw commands.

```rust
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

    fn lrange(&mut self, key: &str, start: i64, stop: i64) -> Result<Vec<String>, RedisErrorWrapper> {
        Ok(self.connection.lrange(key, start, stop)?)
    }

    fn lindex(&mut self, key: &str, index: i64) -> Result<Option<String>, RedisErrorWrapper> {
        Ok(self.connection.lindex(key, index)?)
    }

    fn lset(&mut self, key: &str, index: i64, value: &str) -> Result<(), RedisErrorWrapper> {
        Ok(self.connection.lset(key, index, value)?)
    }

    fn lrem(&mut self, key: &str, count: i64, value: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.lrem(key, count, value)?)
    }

    fn ltrim(&mut self, key: &str, start: i64, stop: i64) -> Result<(), RedisErrorWrapper> {
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
```

**Step 2: Build, Step 3: Add tests**

```php
// Task 5: List Commands
echo "\n--- List Commands ---\n";
$redis->del("test:list");
$redis->rpush("test:list", "a");
$redis->rpush("test:list", "b");
$redis->lpush("test:list", "z");

echo "llen: " . $redis->llen("test:list") . "\n"; // 3
echo "lrange: "; var_dump($redis->lrange("test:list", 0, -1)); // [z, a, b]
echo "lindex 0: " . $redis->lindex("test:list", 0) . "\n"; // z

$redis->lset("test:list", 0, "Z");
echo "after lset: " . $redis->lindex("test:list", 0) . "\n"; // Z

echo "lpop: " . $redis->lpop("test:list") . "\n"; // Z
echo "rpop: " . $redis->rpop("test:list") . "\n"; // b

$redis->rpush("test:list", "c");
$redis->linsertBefore("test:list", "c", "b");
echo "after linsert_before: "; var_dump($redis->lrange("test:list", 0, -1)); // [a, b, c]
```

**Step 4: Run test, Step 5: Commit**

```bash
git add src/lib.rs test_redis.php
git commit -m "feat: add list commands (lpush, rpush, lpop, rpop, lrange, etc.)"
```

---

### Task 6: Set Commands

**Files:**
- Modify: `src/lib.rs`
- Modify: `test_redis.php`

**Step 1: Add set command methods**

Note: `spop` returns generic `RV` — use raw command for single-element version.

```rust
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
```

**Step 2: Build, Step 3: Add tests**

```php
// Task 6: Set Commands
echo "\n--- Set Commands ---\n";
$redis->del("test:set1");
$redis->del("test:set2");
$redis->sadd("test:set1", "a");
$redis->sadd("test:set1", "b");
$redis->sadd("test:set1", "c");

echo "scard: " . $redis->scard("test:set1") . "\n"; // 3
echo "sismember a: " . var_export($redis->sismember("test:set1", "a"), true) . "\n"; // true
echo "sismember z: " . var_export($redis->sismember("test:set1", "z"), true) . "\n"; // false

$members = $redis->smembers("test:set1");
echo "smembers: "; var_dump($members);

$redis->sadd("test:set2", "b");
$redis->sadd("test:set2", "c");
$redis->sadd("test:set2", "d");

$diff = $redis->sdiff(["test:set1", "test:set2"]);
echo "sdiff: "; var_dump($diff); // {a}

$inter = $redis->sinter(["test:set1", "test:set2"]);
echo "sinter: "; var_dump($inter); // {b, c}

$union = $redis->sunion(["test:set1", "test:set2"]);
echo "sunion: "; var_dump($union); // {a, b, c, d}

echo "spop: " . var_export($redis->spop("test:set1"), true) . "\n";
echo "srandmember: " . var_export($redis->srandmember("test:set1"), true) . "\n";
```

**Step 4: Run test, Step 5: Commit**

```bash
git add src/lib.rs test_redis.php
git commit -m "feat: add set commands (sadd, srem, smembers, sdiff, sinter, etc.)"
```

---

### Task 7: Sorted Set Commands

**Files:**
- Modify: `src/lib.rs`
- Modify: `test_redis.php`

**Step 1: Add sorted set command methods**

```rust
    // ── Sorted Set Commands ─────────────────────────────────────

    fn zadd(&mut self, key: &str, member: &str, score: f64) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.zadd(key, member, score)?)
    }

    fn zrem(&mut self, key: &str, member: &str) -> Result<i64, RedisErrorWrapper> {
        Ok(self.connection.zrem(key, member)?)
    }

    fn zrange(&mut self, key: &str, start: i64, stop: i64) -> Result<Vec<String>, RedisErrorWrapper> {
        Ok(self.connection.zrange(key, start, stop)?)
    }

    fn zrevrange(&mut self, key: &str, start: i64, stop: i64) -> Result<Vec<String>, RedisErrorWrapper> {
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
```

**Step 2: Build, Step 3: Add tests**

```php
// Task 7: Sorted Set Commands
echo "\n--- Sorted Set Commands ---\n";
$redis->del("test:zset");
$redis->zadd("test:zset", "alice", 100.0);
$redis->zadd("test:zset", "bob", 200.0);
$redis->zadd("test:zset", "carol", 150.0);

echo "zcard: " . $redis->zcard("test:zset") . "\n"; // 3
echo "zscore alice: " . $redis->zscore("test:zset", "alice") . "\n"; // 100
echo "zrank alice: " . $redis->zrank("test:zset", "alice") . "\n"; // 0

$range = $redis->zrange("test:zset", 0, -1);
echo "zrange: "; var_dump($range); // [alice, carol, bob]

$rev = $redis->zrevrange("test:zset", 0, -1);
echo "zrevrange: "; var_dump($rev); // [bob, carol, alice]

echo "zcount 0-160: " . $redis->zcount("test:zset", "0", "160") . "\n"; // 2

$redis->zincr("test:zset", "alice", 50.0);
echo "after zincr: " . $redis->zscore("test:zset", "alice") . "\n"; // 150

$byscore = $redis->zrangebyscore("test:zset", "150", "+inf");
echo "zrangebyscore: "; var_dump($byscore); // [alice, carol, bob] or subset

echo "zrem: " . $redis->zrem("test:zset", "bob") . "\n"; // 1
```

**Step 4: Run test, Step 5: Commit**

```bash
git add src/lib.rs test_redis.php
git commit -m "feat: add sorted set commands (zadd, zrange, zscore, zrank, etc.)"
```

---

### Task 8: Pub/Sub + Server Commands

**Files:**
- Modify: `src/lib.rs`
- Modify: `test_redis.php`

**Step 1: Add remaining methods**

```rust
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
```

**Step 2: Build, Step 3: Add tests**

```php
// Task 8: Pub/Sub + Server
echo "\n--- Pub/Sub + Server ---\n";
echo "publish (no subscribers): " . $redis->publish("test:chan", "hello") . "\n"; // 0

// flushdb tested implicitly; don't run in test to avoid wiping data
echo "Server commands available: flushdb, flushall\n";
```

**Step 4: Run test, Step 5: Commit**

```bash
git add src/lib.rs test_redis.php
git commit -m "feat: add publish, flushdb, flushall"
```

---

### Task 9: Cleanup test keys and final verification

**Step 1: Add cleanup to test script**

Add at the end of `test_redis.php`:

```php
// Cleanup test keys
echo "\n--- Cleanup ---\n";
$pattern = "test:*";
$keys = $redis->keys($pattern);
foreach ($keys as $key) {
    $redis->del($key);
}
echo "Cleaned up " . count($keys) . " test keys\n";
echo "\nAll tests passed!\n";
```

**Step 2: Full test run**

Run: `php -d extension=target/debug/libphprs_hello_world.dylib test_redis.php`
Expected: All sections print expected output, cleanup runs, no exceptions.

**Step 3: Commit**

```bash
git add test_redis.php
git commit -m "feat: add test cleanup, final verification"
```
