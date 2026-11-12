# RustFS Performance Test Results

- Date: 2026-11-12 03:57:46 UTC
- Target: rustfs-node1:9000,rustfs-node2:9000,rustfs-node3:9000,rustfs-node4:9000
- Concurrency: 64; duration: 5m; sleep: 60s; GET objects: 2500
- Methods: get put mixed; sizes: 4KiB 16KiB 128KiB 1MiB 4MiB 8MiB 16MiB 32MiB 64MiB
- Rounds: selected=27; parsed=5; failed=0

## GET Results

| Object Size | Avg Throughput | Avg obj/s | Avg Latency | P50 | P90 | P99 | TTFB Avg | TTFB P99 | TTFB Worst |
|-------------|----------------|-----------|-------------|-----|-----|-----|----------|----------|------------|
| 4KiB | 29.13 MiB/s | 7456.17 | 8.6ms | 8.5ms | 10.5ms | 12.6ms | 9ms | 13ms | 55ms |
| 16KiB | 114.78 MiB/s | 7345.90 | 8.7ms | 8.6ms | 10.6ms | 12.8ms | 9ms | 13ms | 64ms |
| 128KiB | 847.07 MiB/s | 6776.54 | 9.4ms | 9.3ms | 11.6ms | 14.0ms | 9ms | 14ms | 78ms |
| 1MiB | 2854.41 MiB/s | 2854.41 | 22.4ms | 22.2ms | 26.8ms | 31.4ms | 20ms | 29ms | 66ms |
| 4MiB | 5272.41 MiB/s | 1318.10 | 48.6ms | 48.7ms | 56.9ms | 65.2ms | 32ms | 50ms | 203ms |

## PUT Results

| Object Size | Avg Throughput | Avg obj/s | Avg Latency | P50 | P90 | P99 | TTFB Avg | TTFB P99 | TTFB Worst |
|-------------|----------------|-----------|-------------|-----|-----|-----|----------|----------|------------|

## MIXED Results (Total)

| Object Size | Total Avg Throughput | Total Avg obj/s | MIXED GET Avg | MIXED PUT Avg |
|-------------|----------------------|-----------------|---------------|---------------|

