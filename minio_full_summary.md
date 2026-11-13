# RustFS Performance Test Results

- Date: 2026-11-13 15:48:49 UTC
- Target: rustfs-node1:9000,rustfs-node2:9000,rustfs-node3:9000,rustfs-node4:9000
- Concurrency: 64; duration: 60s; sleep: 15s; GET objects: 2500
- Methods: get put mixed; sizes: 1KiB 4KiB 16KiB 128KiB 1MiB 4MiB 8MiB 16MiB 32MiB 64MiB
- Rounds: selected=30; parsed=8; failed=0

## GET Results

| Object Size | Avg Throughput | Avg obj/s | Avg Latency | P50 | P90 | P99 | TTFB Avg | TTFB P99 | TTFB Worst |
|-------------|----------------|-----------|-------------|-----|-----|-----|----------|----------|------------|
| 1KiB | 9.33 MiB/s | 9551.12 | 6.7ms | 6.0ms | 10.0ms | 16.8ms | 7ms | 17ms | 228ms |
| 4KiB | 35.74 MiB/s | 9148.94 | 6.9ms | 6.2ms | 10.2ms | 17.8ms | 7ms | 17ms | 229ms |
| 16KiB | 147.95 MiB/s | 9468.71 | 6.7ms | 6.0ms | 9.9ms | 16.7ms | 7ms | 17ms | 57ms |
| 128KiB | 935.66 MiB/s | 7485.29 | 8.5ms | 7.7ms | 13.0ms | 20.9ms | 8ms | 21ms | 45ms |
| 1MiB | 3905.35 MiB/s | 3905.35 | 16.5ms | 15.4ms | 24.7ms | 34.0ms | 15ms | 32ms | 212ms |
| 4MiB | 7767.41 MiB/s | 1941.85 | 33.0ms | 33.0ms | 38.5ms | 45.3ms | 20ms | 32ms | 55ms |
| 8MiB | 8785.97 MiB/s | 1098.25 | 58.2ms | 59.5ms | 64.6ms | 70.5ms | 22ms | 33ms | 260ms |
| 16MiB | 9500.42 MiB/s | 593.78 | 107.5ms | 112.2ms | 117.4ms | 123.1ms | 21ms | 33ms | 54ms |

## PUT Results

| Object Size | Avg Throughput | Avg obj/s | Avg Latency | P50 | P90 | P99 | TTFB Avg | TTFB P99 | TTFB Worst |
|-------------|----------------|-----------|-------------|-----|-----|-----|----------|----------|------------|

## MIXED Results (Total)

| Object Size | Total Avg Throughput | Total Avg obj/s | MIXED GET Avg | MIXED PUT Avg |
|-------------|----------------------|-----------------|---------------|---------------|

