# RustFS Performance Test Results

- Date: 2026-11-12 04:26:19 UTC
- Target: rustfs-node1:9000,rustfs-node2:9000,rustfs-node3:9000,rustfs-node4:9000
- Concurrency: 64; duration: 5m; sleep: 60s; GET objects: 2500
- Methods: get put mixed; sizes: 4KiB 16KiB 128KiB 1MiB 4MiB 8MiB 16MiB 32MiB 64MiB
- Rounds: selected=27; parsed=9; failed=0

## GET Results

| Object Size | Avg Throughput | Avg obj/s | Avg Latency | P50 | P90 | P99 | TTFB Avg | TTFB P99 | TTFB Worst |
|-------------|----------------|-----------|-------------|-----|-----|-----|----------|----------|------------|
| 4KiB | 29.13 MiB/s | 7456.17 | 8.6ms | 8.5ms | 10.5ms | 12.6ms | 9ms | 13ms | 55ms |
| 16KiB | 114.78 MiB/s | 7345.90 | 8.7ms | 8.6ms | 10.6ms | 12.8ms | 9ms | 13ms | 64ms |
| 128KiB | 847.07 MiB/s | 6776.54 | 9.4ms | 9.3ms | 11.6ms | 14.0ms | 9ms | 14ms | 78ms |
| 1MiB | 2854.41 MiB/s | 2854.41 | 22.4ms | 22.2ms | 26.8ms | 31.4ms | 20ms | 29ms | 66ms |
| 4MiB | 5272.41 MiB/s | 1318.10 | 48.6ms | 48.7ms | 56.9ms | 65.2ms | 32ms | 50ms | 203ms |
| 8MiB | 5591.96 MiB/s | 699.00 | 91.5ms | 92.0ms | 103.8ms | 116.3ms | 45ms | 68ms | 172ms |
| 16MiB | 6424.71 MiB/s | 401.54 | 159.2ms | 160.3ms | 171.4ms | 187.5ms | 53ms | 86ms | 469ms |
| 32MiB | 5764.90 MiB/s | 180.15 | 351.8ms | 352.8ms | 378.5ms | 411.5ms | 84ms | 153ms | 735ms |
| 64MiB | 6483.03 MiB/s | 101.30 | 632.2ms | 628.7ms | 699.6ms | 775.2ms | 82ms | 159ms | 340ms |

## PUT Results

| Object Size | Avg Throughput | Avg obj/s | Avg Latency | P50 | P90 | P99 | TTFB Avg | TTFB P99 | TTFB Worst |
|-------------|----------------|-----------|-------------|-----|-----|-----|----------|----------|------------|

## MIXED Results (Total)

| Object Size | Total Avg Throughput | Total Avg obj/s | MIXED GET Avg | MIXED PUT Avg |
|-------------|----------------------|-----------------|---------------|---------------|

