<?php
$redis = new Redis\Client("redis://127.0.0.1");
$redis->flushAll();
$start = microtime(true);
for ($i = 0; $i < 100_000; $i++) {
    $redis->set("test:key$i", "value$i");
    $value = $redis->get("test:key$i");
    assert($value === "value$i", "Value mismatch for key test:key$i");
}
$end = microtime(true);
echo "PHPRS_redis Time taken: " . ($end - $start) . " seconds" .PHP_EOL;

$redis = new Redis([
    'host' => '127.0.0.1',
    'port' => 6379]);

$redis->flushAll();

$start = microtime(true);
for ($i = 0; $i < 100_000; $i++) {
    $redis->set("test:key$i", "value$i");
    $value = $redis->get("test:key$i");
    assert($value === "value$i", "Value mismatch for key test:key$i");
}
$end = microtime(true);
echo "PHPredis Time taken: " . ($end - $start) . " seconds" .PHP_EOL;
