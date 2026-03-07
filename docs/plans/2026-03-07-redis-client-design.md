# Redis Client PHP Extension - Design

## Goal

Wrap the Rust `redis` crate (v1.0.4) via `ext-php-rs` to provide a PHP Redis client extension. Write methods directly (no macros). Cover as much of the `TypedCommands` trait as possible, skipping commands with complex return types initially.

## Architecture

Single file (`src/lib.rs`) containing:

- A `RedisException` PHP exception class extending `\Exception`
- A `RedisClient` PHP class with a persistent `redis::Connection`
- All command methods in one `#[php_impl]` block (ext-php-rs limitation: one impl block per class)
- Command groups separated by comment sections

## Connection Management

Store a persistent `redis::Connection` in the struct (not `redis::Client`). One TCP connection per PHP object lifetime.

```rust
#[php_class]
#[php(name = "Redis\\Client")]
struct RedisClient {
    connection: redis::Connection,
}
```

Constructor:
```rust
fn __construct(dsn: &str) -> Result<Self, PhpRedisException> {
    let client = redis::Client::open(dsn)?;
    let connection = client.get_connection()?;
    Ok(Self { connection })
}
```

## Error Handling

Typed PHP exception class:

```rust
#[php_class]
#[php(name = "Redis\\RedisException")]
#[php(extends(ce = ce::exception, stub = "\\Exception"))]
#[derive(Default)]
pub struct PhpRedisException;
```

Bridge from `redis::RedisError`:

```rust
struct RedisErrorWrapper(RedisError);

impl From<RedisError> for RedisErrorWrapper { ... }
impl From<RedisErrorWrapper> for PhpException {
    fn from(value: RedisErrorWrapper) -> Self {
        PhpException::from_class::<PhpRedisException>(value.0.to_string())
    }
}
```

## Type Mapping

| Redis/Rust return type | PHP type | Example commands |
|---|---|---|
| `RedisResult<Option<String>>` | `?string` | GET, HGET, GETSET |
| `RedisResult<()>` | `void` | SET, MSET, LTRIM |
| `RedisResult<bool>` | `bool` | EXISTS, SETNX, EXPIRE |
| `RedisResult<usize>` | `int` | DEL, LPUSH, SADD |
| `RedisResult<isize>` | `int` | INCR, DECR, LINSERT |
| `RedisResult<f64>` | `float` | ZSCORE, HINCRBYFLOAT |
| `RedisResult<Vec<String>>` | `string[]` | KEYS, LRANGE, MGET |
| `RedisResult<HashMap<String,String>>` | `array<string,string>` | HGETALL |
| `RedisResult<HashSet<String>>` | `string[]` | SMEMBERS, SDIFF, SINTER |
| `RedisResult<Option<usize>>` | `?int` | ZRANK, ZREVRANK |
| `RedisResult<Option<f64>>` | `?float` | ZSCORE |

## Command Groups (implementation order)

### 1. Connection
- `__construct(dsn)`, `ping()`

### 2. String Commands
- `get`, `set`, `set_ex`, `set_nx`, `mget`, `mset`, `incr`, `decr`, `append`, `strlen`, `getset`, `getrange`, `setrange`, `get_del`

### 3. Key Commands
- `del`, `exists`, `expire`, `expire_at`, `ttl`, `pttl`, `persist`, `keys`, `rename`, `rename_nx`, `unlink`

### 4. Hash Commands
- `hget`, `hset`, `hdel`, `hgetall`, `hkeys`, `hvals`, `hexists`, `hlen`, `hset_nx`, `hincr`, `hmget`, `hset_multiple`

### 5. List Commands
- `lpush`, `rpush`, `lpop`, `rpop`, `llen`, `lrange`, `lindex`, `lset`, `lrem`, `ltrim`, `linsert_before`, `linsert_after`

### 6. Set Commands
- `sadd`, `srem`, `smembers`, `sismember`, `scard`, `sdiff`, `sinter`, `sunion`, `smove`, `spop`, `srandmember`

### 7. Sorted Set Commands
- `zadd`, `zrem`, `zrange`, `zrank`, `zscore`, `zcard`, `zcount`, `zincr`, `zrangebyscore`, `zrevrange`, `zrevrank`

### 8. Pub/Sub
- `publish` only (subscribe requires async, skipped)

### 9. Server
- `flushdb`, `flushall`

## Skipped (complex types - track for future)

- **Streams**: `xadd`, `xread`, `xrange`, etc. - return `StreamReadReply`, `StreamRangeReply`
- **Scan iterators**: `scan`, `hscan`, `sscan`, `zscan` - return `Iter<'_>`
- **Scripting**: `load_script`, `invoke_script` - complex invocation types
- **Vector ops**: `vadd`, `vsim`, etc. - specialized types
- **Geo commands**: `geo_radius`, `geo_pos` - return `Coord<f64>`, `RadiusSearchResult`
- **Blocking ops with tuples**: `blpop`, `brpop` - return `Option<[String; 2]>`
- **Sorted set with scores**: `zrange_withscores` - returns `Vec<(String, f64)>`
- **ACL commands**: `acl_getuser` - returns `AclInfo`
- **TTL commands**: `ttl`, `pttl` - return `IntegerReplyOrNoOp` (need custom mapping)

## PHP Usage Example

```php
<?php
use Redis\Client;
use Redis\RedisException;

try {
    $redis = new Client("redis://localhost");

    $redis->set("name", "world");
    echo $redis->get("name"); // "world"

    $redis->hset("user:1", "name", "Alice");
    $all = $redis->hgetall("user:1"); // ["name" => "Alice"]

    $redis->lpush("queue", "job1");
    $redis->lpush("queue", "job2");
    $jobs = $redis->lrange("queue", 0, -1); // ["job2", "job1"]

} catch (RedisException $e) {
    echo "Redis error: " . $e->getMessage();
}
```
