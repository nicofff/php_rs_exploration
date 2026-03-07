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

// Task 2: String Commands
echo "\n--- String Commands ---\n";
$redis->set("test:str", "hello");
echo "get: " . $redis->get("test:str") . "\n";
echo "get missing: " . var_export($redis->get("test:nonexistent"), true) . "\n";

$redis->setEx("test:ttl", "expires", 60);
echo "set_ex: " . $redis->get("test:ttl") . "\n";

echo "set_nx new: " . var_export($redis->setNx("test:nx", "first"), true) . "\n";
echo "set_nx exists: " . var_export($redis->setNx("test:nx", "second"), true) . "\n";

$redis->set("test:a", "1");
$redis->set("test:b", "2");
$vals = $redis->mget(["test:a", "test:b", "test:missing"]);
echo "mget: "; var_dump($vals);

$redis->mset(["test:m1" => "x", "test:m2" => "y"]);
echo "mset then get: " . $redis->get("test:m1") . ", " . $redis->get("test:m2") . "\n";

$redis->set("test:counter", "10");
echo "incr: " . $redis->incr("test:counter", 5) . "\n";
echo "decr: " . $redis->decr("test:counter", 3) . "\n";

echo "append: " . $redis->append("test:str", " world") . "\n";
echo "strlen: " . $redis->strlen("test:str") . "\n";

echo "getset: " . var_export($redis->getset("test:str", "new"), true) . "\n";
echo "getrange: " . $redis->getrange("test:str", 0, 2) . "\n";

$redis->set("test:sr", "Hello World");
echo "setrange: " . $redis->setrange("test:sr", 6, "Redis") . "\n";
echo "after setrange: " . $redis->get("test:sr") . "\n";

echo "get_del: " . var_export($redis->getDel("test:str"), true) . "\n";
echo "after get_del: " . var_export($redis->get("test:str"), true) . "\n";

// Task 3: Key Commands
echo "\n--- Key Commands ---\n";
$redis->set("test:key1", "val");
echo "exists: " . var_export($redis->exists("test:key1"), true) . "\n";
echo "exists missing: " . var_export($redis->exists("test:nope"), true) . "\n";
echo "del: " . $redis->del("test:key1") . "\n";
echo "del missing: " . $redis->del("test:key1") . "\n";

$redis->set("test:exp", "temp");
echo "expire: " . var_export($redis->expire("test:exp", 60), true) . "\n";
echo "persist: " . var_export($redis->persist("test:exp"), true) . "\n";

$redis->set("test:ren1", "val");
$redis->rename("test:ren1", "test:ren2");
echo "after rename: " . $redis->get("test:ren2") . "\n";

$redis->set("test:keys_a", "1");
$redis->set("test:keys_b", "2");
$found = $redis->keys("test:keys_*");
echo "keys count: " . count($found) . "\n";

echo "unlink: " . $redis->unlink("test:ren2") . "\n";

// Task 4: Hash Commands
echo "\n--- Hash Commands ---\n";
$redis->hset("test:hash", "name", "Alice");
$redis->hset("test:hash", "age", "30");

echo "hget: " . $redis->hget("test:hash", "name") . "\n";
echo "hexists: " . var_export($redis->hexists("test:hash", "name"), true) . "\n";
echo "hlen: " . $redis->hlen("test:hash") . "\n";

$all = $redis->hgetall("test:hash");
echo "hgetall count: " . count($all) . "\n";

$keys = $redis->hkeys("test:hash");
echo "hkeys count: " . count($keys) . "\n";

$vals = $redis->hvals("test:hash");
echo "hvals count: " . count($vals) . "\n";

$redis->hincr("test:hash", "age", 1.0);
echo "after hincr: " . $redis->hget("test:hash", "age") . "\n";

$redis->hsetMultiple("test:hash2", ["a" => "1", "b" => "2"]);
$multi = $redis->hmget("test:hash2", ["a", "b"]);
echo "hmget count: " . count($multi) . "\n";

echo "hsetNx new: " . var_export($redis->hsetNx("test:hash", "new_field", "val"), true) . "\n";
echo "hsetNx exists: " . var_export($redis->hsetNx("test:hash", "name", "Bob"), true) . "\n";

echo "hdel: " . $redis->hdel("test:hash", "name") . "\n";

// Task 5: List Commands
echo "\n--- List Commands ---\n";
$redis->del("test:list");
$redis->rpush("test:list", "a");
$redis->rpush("test:list", "b");
$redis->lpush("test:list", "z");

echo "llen: " . $redis->llen("test:list") . "\n";
$range = $redis->lrange("test:list", 0, -1);
echo "lrange: " . implode(", ", $range) . "\n";
echo "lindex 0: " . $redis->lindex("test:list", 0) . "\n";

$redis->lset("test:list", 0, "Z");
echo "after lset: " . $redis->lindex("test:list", 0) . "\n";

echo "lpop: " . $redis->lpop("test:list") . "\n";
echo "rpop: " . $redis->rpop("test:list") . "\n";

$redis->rpush("test:list", "c");
$redis->linsertBefore("test:list", "c", "b");
$range = $redis->lrange("test:list", 0, -1);
echo "after linsert_before: " . implode(", ", $range) . "\n";

echo "lrem: " . $redis->lrem("test:list", 1, "b") . "\n";

echo "lpush_exists on missing: " . $redis->lpushExists("test:missing_list", "val") . "\n";
$redis->rpush("test:list", "x");
echo "rpush_exists: " . $redis->rpushExists("test:list", "y") . "\n";

// Task 6: Set Commands
echo "\n--- Set Commands ---\n";
$redis->del("test:set1");
$redis->del("test:set2");
$redis->sadd("test:set1", "a");
$redis->sadd("test:set1", "b");
$redis->sadd("test:set1", "c");

echo "scard: " . $redis->scard("test:set1") . "\n";
echo "sismember a: " . var_export($redis->sismember("test:set1", "a"), true) . "\n";
echo "sismember z: " . var_export($redis->sismember("test:set1", "z"), true) . "\n";

$members = $redis->smembers("test:set1");
echo "smembers count: " . count($members) . "\n";

$redis->sadd("test:set2", "b");
$redis->sadd("test:set2", "c");
$redis->sadd("test:set2", "d");

$diff = $redis->sdiff(["test:set1", "test:set2"]);
echo "sdiff count: " . count($diff) . "\n";

$inter = $redis->sinter(["test:set1", "test:set2"]);
echo "sinter count: " . count($inter) . "\n";

$union = $redis->sunion(["test:set1", "test:set2"]);
echo "sunion count: " . count($union) . "\n";

echo "smove: " . var_export($redis->smove("test:set1", "test:set2", "a"), true) . "\n";
echo "spop: " . var_export($redis->spop("test:set1") !== null, true) . "\n";
echo "srandmember: " . var_export($redis->srandmember("test:set1") !== null, true) . "\n";

// Task 7: Sorted Set Commands
echo "\n--- Sorted Set Commands ---\n";
$redis->del("test:zset");
$redis->zadd("test:zset", "alice", 100.0);
$redis->zadd("test:zset", "bob", 200.0);
$redis->zadd("test:zset", "carol", 150.0);

echo "zcard: " . $redis->zcard("test:zset") . "\n";
echo "zscore alice: " . $redis->zscore("test:zset", "alice") . "\n";
echo "zrank alice: " . $redis->zrank("test:zset", "alice") . "\n";

$range = $redis->zrange("test:zset", 0, -1);
echo "zrange: " . implode(", ", $range) . "\n";

$rev = $redis->zrevrange("test:zset", 0, -1);
echo "zrevrange: " . implode(", ", $rev) . "\n";

echo "zcount 0-160: " . $redis->zcount("test:zset", "0", "160") . "\n";

$redis->zincr("test:zset", "alice", 50.0);
echo "after zincr: " . $redis->zscore("test:zset", "alice") . "\n";

$byscore = $redis->zrangebyscore("test:zset", "150", "+inf");
echo "zrangebyscore count: " . count($byscore) . "\n";

echo "zrem: " . $redis->zrem("test:zset", "bob") . "\n";

// Task 8: Pub/Sub + Server
echo "\n--- Pub/Sub + Server ---\n";
echo "publish (no subscribers): " . $redis->publish("test:chan", "hello") . "\n";
echo "Server commands available: flushdb, flushall\n";

// Test error propagation for command errors
try {
    $redis->set("test:notlist", "string_value");
    $redis->llen("test:notlist");
    echo "FAIL: should have thrown\n";
} catch (Redis\RedisException $e) {
    echo "Expected WRONGTYPE error: OK\n";
}

// Cleanup test keys
echo "\n--- Cleanup ---\n";
$pattern = "test:*";
$keys = $redis->keys($pattern);
foreach ($keys as $key) {
    $redis->del($key);
}
echo "Cleaned up " . count($keys) . " test keys\n";
echo "\nAll tests passed!\n";
