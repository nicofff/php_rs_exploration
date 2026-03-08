<?php

$redis = new RedisWrapper("redis://localhost");
$key = "my_key2";
$value = $redis->getKey($key);
var_dump($value);
