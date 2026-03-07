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
