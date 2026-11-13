# RustFS Performance Test Results

- Date: 2026-11-13 16:25:40 UTC
- Target: rustfs-node1:9000,rustfs-node2:9000,rustfs-node3:9000,rustfs-node4:9000
- Concurrency: 64; duration: 60s; sleep: 15s; GET objects: 2500
- Methods: get put mixed; sizes: 1KiB 4KiB 16KiB 128KiB 1MiB 4MiB 8MiB 16MiB 32MiB 64MiB
- Rounds: selected=30; parsed=30; failed=0

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
| 32MiB | 9612.00 MiB/s | 300.37 | 212.3ms | 220.0ms | 232.4ms | 245.6ms | 24ms | 39ms | 65ms |
| 64MiB | 9597.50 MiB/s | 149.96 | 423.0ms | 425.2ms | 444.2ms | 463.8ms | 25ms | 41ms | 67ms |

## PUT Results

| Object Size | Avg Throughput | Avg obj/s | Avg Latency | P50 | P90 | P99 | TTFB Avg | TTFB P99 | TTFB Worst |
|-------------|----------------|-----------|-------------|-----|-----|-----|----------|----------|------------|
| 1KiB | 1.62 MiB/s | 1655.88 | 40.8ms | 5.0ms | 183.4ms | 339.3ms | 41ms | 339ms | 435ms |
| 4KiB | 6.65 MiB/s | 1702.39 | 38.4ms | 5.1ms | 171.1ms | 338.6ms | 38ms | 339ms | 428ms |
| 16KiB | 26.11 MiB/s | 1671.16 | 46.3ms | 6.8ms | 187.0ms | 340.8ms | 46ms | 341ms | 569ms |
| 128KiB | 237.51 MiB/s | 1900.08 | 36.2ms | 7.0ms | 144.7ms | 337.5ms | 36ms | 337ms | 372ms |
| 1MiB | 1231.01 MiB/s | 1231.01 | 55.3ms | 23.6ms | 174.8ms | 357.5ms | 54ms | 357ms | 394ms |
| 4MiB | 2549.45 MiB/s | 637.36 | 108.8ms | 106.0ms | 137.3ms | 177.8ms | 67ms | 113ms | 434ms |
| 8MiB | 3457.99 MiB/s | 432.25 | 144.7ms | 142.2ms | 179.2ms | 222.6ms | 73ms | 124ms | 369ms |
| 16MiB | 4128.29 MiB/s | 258.02 | 246.4ms | 249.2ms | 303.1ms | 369.3ms | 91ms | 162ms | 290ms |
| 32MiB | 3299.85 MiB/s | 103.12 | 620.2ms | 587.2ms | 829.8ms | 1014.6ms | 363ms | 741ms | 910ms |
| 64MiB | 2697.35 MiB/s | 42.15 | 1481.1ms | 1468.6ms | 1828.0ms | 2234.7ms | 853ms | 1.479s | 2.357s |

## MIXED Results (Total)

| Object Size | Total Avg Throughput | Total Avg obj/s | MIXED GET Avg | MIXED PUT Avg |
|-------------|----------------------|-----------------|---------------|---------------|
| 1KiB | 2.75 MiB/s | 4696.26 | 2.2ms | 80.0ms |
| 4KiB | 11.59 MiB/s | 4945.94 | 2.5ms | 73.1ms |
| 16KiB | 47.08 MiB/s | 5021.47 | 2.2ms | 73.6ms |
| 128KiB | 322.33 MiB/s | 4297.72 | 5.2ms | 82.8ms |
| 1MiB | 1900.04 MiB/s | 3166.66 | 10.9ms | 71.3ms |
| 4MiB | 4686.78 MiB/s | 1952.63 | 30.3ms | 69.2ms |
| 8MiB | 6177.87 MiB/s | 1287.14 | 52.8ms | 103.8ms |
| 16MiB | 7021.76 MiB/s | 731.32 | 103.6ms | 182.1ms |
| 32MiB | 7094.82 MiB/s | 369.48 | 197.8ms | 452.4ms |
| 64MiB | 6918.64 MiB/s | 180.06 | 492.3ms | 683.9ms |

