# RustFS Performance Test Results

- Date: 2026-11-12 07:32:19 UTC
- Target: rustfs-node1:9000,rustfs-node2:9000,rustfs-node3:9000,rustfs-node4:9000
- Concurrency: 64; duration: 5m; sleep: 60s; GET objects: 2500
- Methods: get put mixed; sizes: 1KiB 4KiB 16KiB 128KiB 1MiB 4MiB 8MiB 16MiB 32MiB 64MiB
- Rounds: selected=30; parsed=10; failed=0

## GET Results

| Object Size | Avg Throughput | Avg obj/s | Avg Latency | P50 | P90 | P99 | TTFB Avg | TTFB P99 | TTFB Worst |
|-------------|----------------|-----------|-------------|-----|-----|-----|----------|----------|------------|
| 1KiB | 7.30 MiB/s | 7477.28 | 8.5ms | 8.3ms | 10.6ms | 13.4ms | 9ms | 13ms | 254ms |
| 4KiB | 29.54 MiB/s | 7562.30 | 8.4ms | 8.3ms | 10.3ms | 12.9ms | 8ms | 13ms | 100ms |
| 16KiB | 116.85 MiB/s | 7478.17 | 8.5ms | 8.4ms | 10.4ms | 12.6ms | 9ms | 13ms | 79ms |
| 128KiB | 864.32 MiB/s | 6914.53 | 9.3ms | 9.1ms | 11.3ms | 14.0ms | 9ms | 14ms | 54ms |
| 1MiB | 2855.58 MiB/s | 2855.58 | 22.4ms | 22.1ms | 26.9ms | 31.9ms | 20ms | 29ms | 336ms |
| 4MiB | 4847.15 MiB/s | 1211.79 | 52.8ms | 52.8ms | 62.7ms | 72.2ms | 38ms | 59ms | 266ms |
| 8MiB | 5356.45 MiB/s | 669.56 | 95.5ms | 95.8ms | 109.0ms | 122.3ms | 51ms | 79ms | 372ms |
| 16MiB | 6295.86 MiB/s | 393.49 | 162.4ms | 163.5ms | 177.1ms | 202.0ms | 60ms | 96ms | 431ms |
| 32MiB | 5772.89 MiB/s | 180.40 | 352.9ms | 356.3ms | 386.4ms | 422.5ms | 96ms | 167ms | 409ms |
| 64MiB | 6441.50 MiB/s | 100.65 | 633.2ms | 634.8ms | 709.1ms | 780.1ms | 90ms | 170ms | 374ms |

## PUT Results

| Object Size | Avg Throughput | Avg obj/s | Avg Latency | P50 | P90 | P99 | TTFB Avg | TTFB P99 | TTFB Worst |
|-------------|----------------|-----------|-------------|-----|-----|-----|----------|----------|------------|

## MIXED Results (Total)

| Object Size | Total Avg Throughput | Total Avg obj/s | MIXED GET Avg | MIXED PUT Avg |
|-------------|----------------------|-----------------|---------------|---------------|

