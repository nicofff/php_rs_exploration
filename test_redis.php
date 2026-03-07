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
