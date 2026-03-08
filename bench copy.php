<?php
// Simple and robust benchmark using only the available PHP Redis extension

function runBenchmark($redisClient, $clientName, $iterations = 100000) {
    echo "Running benchmark for $clientName...\n";
    
    // Warm-up run
    try {
        $redisClient->flushAll();
        for ($i = 0; $i < 1000; $i++) {
            $redisClient->set("warmup:key$i", "value$i");
            $redisClient->get("warmup:key$i");
        }
    } catch (Exception $e) {
        echo "Warm-up failed for $clientName: " . $e->getMessage() . "\n";
        return null;
    }
    
    $results = [];
    
    // Run multiple iterations to get average performance
    for ($run = 0; $run < 3; $run++) {
        try {
            $redisClient->flushAll();
            
            $start = microtime(true);
            $startMemory = memory_get_usage();
            
            for ($i = 0; $i < $iterations; $i++) {
                $redisClient->set("test:key$i", "value$i");
                $value = $redisClient->get("test:key$i");
                if ($value !== "value$i") {
                    throw new Exception("Value mismatch for key test:key$i");
                }
            }
            
            $end = microtime(true);
            $endMemory = memory_get_usage();
            
            $elapsed = $end - $start;
            $memoryUsed = $endMemory - $startMemory;
            $opsPerSecond = $iterations / $elapsed;
            
            $results[] = [
                'time' => $elapsed,
                'memory' => $memoryUsed,
                'ops_per_second' => $opsPerSecond
            ];
            
            echo "Run $run: {$elapsed} seconds, {$opsPerSecond} ops/sec, " . 
                 number_format($memoryUsed / 1024, 2) . " KB memory\n";
                 
        } catch (Exception $e) {
            echo "Run $run failed for $clientName: " . $e->getMessage() . "\n";
            continue;
        }
    }
    
    if (empty($results)) {
        echo "All runs failed for $clientName\n";
        return null;
    }
    
    // Calculate averages
    $avgTime = array_sum(array_column($results, 'time')) / count($results);
    $avgMemory = array_sum(array_column($results, 'memory')) / count($results);
    $avgOpsPerSecond = array_sum(array_column($results, 'ops_per_second')) / count($results);
    
    return [
        'avg_time' => $avgTime,
        'avg_memory' => $avgMemory,
        'avg_ops_per_second' => $avgOpsPerSecond,
        'results' => $results
    ];
}

// Test with standard PHP Redis extension
echo "=== Testing PHP Redis Extension ===\n";
try {
    $redis = new Redis();
    $redis->connect('127.0.0.1', 6379);
    
    $results = runBenchmark($redis, "PHP Redis Extension", 100000);
    
    if ($results !== null) {
        echo "\nPHP Redis Extension Results:\n";
        echo "Average time: " . number_format($results['avg_time'], 4) . " seconds\n";
        echo "Average ops/sec: " . number_format($results['avg_ops_per_second'], 2) . "\n";
        echo "Average memory usage: " . number_format($results['avg_memory'] / 1024, 2) . " KB\n";
    } else {
        echo "PHP Redis Extension benchmark failed completely\n";
    }
    
} catch (Exception $e) {
    echo "PHP Redis Extension test failed: " . $e->getMessage() . "\n";
}

echo "\n" . str_repeat("-", 50) . "\n\n";

// Test with Redis cluster if available
echo "=== Testing Rust Extension ===\n";
try {
    $redisRust = new \Redis\Client("redis://localhost");
    $clusterResults = runBenchmark($redisRust, "Redis Rust", 100000);
    
    if ($clusterResults !== null) {
        echo "\nRedis Cluster Results:\n";
        echo "Average time: " . number_format($clusterResults['avg_time'], 4) . " seconds\n";
        echo "Average ops/sec: " . number_format($clusterResults['avg_ops_per_second'], 2) . "\n";
        echo "Average memory usage: " . number_format($clusterResults['avg_memory'] / 1024, 2) . " KB\n";
    } else {
        echo "Redis Cluster benchmark failed completely\n";
    }
    
} catch (Exception $e) {
    echo "Redis Cluster test not available or failed: " . $e->getMessage() . "\n";
}

echo "\nBenchmark completed.\n";