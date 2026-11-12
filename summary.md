# RustFS Performance Test Results

- Date: 2026-11-12 06:37:32 UTC
- Target: rustfs-node1:9000,rustfs-node2:9000,rustfs-node3:9000,rustfs-node4:9000
- Concurrency: 64; duration: 5m; sleep: 60s; GET objects: 2500
- Methods: get put mixed; sizes: 1KiB 4KiB 16KiB 128KiB 1MiB 4MiB 8MiB 16MiB 32MiB 64MiB
- Rounds: selected=30; parsed=2; failed=0

## GET Results

| Object Size | Avg Throughput | Avg obj/s | Avg Latency | P50 | P90 | P99 | TTFB Avg | TTFB P99 | TTFB Worst |
|-------------|----------------|-----------|-------------|-----|-----|-----|----------|----------|------------|
| 1KiB | 7.30 MiB/s | 7477.28 | 8.5ms | 8.3ms | 10.6ms | 13.4ms | 9ms | 13ms | 254ms |
| 4KiB | 29.54 MiB/s | 7562.30 | 8.4ms | 8.3ms | 10.3ms | 12.9ms | 8ms | 13ms | 100ms |

## PUT Results

| Object Size | Avg Throughput | Avg obj/s | Avg Latency | P50 | P90 | P99 | TTFB Avg | TTFB P99 | TTFB Worst |
|-------------|----------------|-----------|-------------|-----|-----|-----|----------|----------|------------|

## MIXED Results (Total)

| Object Size | Total Avg Throughput | Total Avg obj/s | MIXED GET Avg | MIXED PUT Avg |
|-------------|----------------------|-----------------|---------------|---------------|

