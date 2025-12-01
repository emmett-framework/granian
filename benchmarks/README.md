# Granian benchmarks



Run at: Mon 01 Dec 2025, 16:13    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.12.58 (CPUs: 16)    
Python version: 3.13    
Granian version: 2.6.0

### Methodology

Unless otherwise specified in the specific benchmark section, Granian is run:

- Using default configuration, thus:
  - 1 worker
  - 1 runtime thread
- With `--runtime-mode` set to `st` on ASGI and `mt` otherwise
- With `--http 1` flag
- With `--no-ws` flag
- With `uvloop` event-loop on async protocols

Tests are peformed using `oha` utility, with the concurrency specified in the specific test. The test run for 10 seconds, preceeded by a *primer* run at concurrency 8 for 4 seconds, and a *warmup* run at the maximum configured concurrency for the test for 3 seconds.

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes 10B (c64) | 1473133 | 147247 | 0.433ms | 36.05ms |
| str 10B (c64) | 1487132 | 148650 | 0.429ms | 20.978ms |
| bytes 100KB (c64) | 567227 | 56711 | 1.124ms | 43.169ms |
| str 100KB (c64) | 588868 | 58876 | 1.084ms | 22.076ms |


## Interfaces

Comparison between Granian application protocols using plain text responses.

The 1KB GET and POST tests are run with `--blocking-threads 1`.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI get 1KB (c128) | 1478812 | 147828 | 0.863ms | 46.558ms |
| RSGI echo 1KB (c128) | 1192967 | 119250 | 1.069ms | 63.689ms |
| RSGI echo 100KB (iter) (c64) | 176773 | 17680 | 3.607ms | 59.939ms |
| ASGI get 1KB (c128) | 1210305 | 121000 | 1.053ms | 67.592ms |
| ASGI echo 1KB (c128) | 778324 | 77817 | 1.639ms | 52.452ms |
| ASGI echo 100KB (iter) (c64) | 196408 | 19642 | 3.245ms | 68.731ms |
| WSGI get 1KB (c64) | 1462067 | 146147 | 0.436ms | 26.963ms |
| WSGI echo 1KB (c64) | 1330240 | 132975 | 0.479ms | 40.363ms |
| WSGI echo 100KB (iter) (c64) | 93828 | 9387 | 6.8ms | 52.449ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using plain text responses.

All tests are run with `--runtime-threads 2`.
HTTP/2 tests are run with `--http 2`.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 get 1KB (c128) | 2069053 | 206831 | 0.616ms | 57.395ms |
| HTTP/1 echo 1KB (c128) | 1225556 | 122519 | 1.04ms | 66.926ms |
| HTTP/2 get 1KB (c128) | 1853067 | 185274 | 2.748ms | 8.315ms |
| HTTP/2 echo 1KB (c128) | 1138862 | 113864 | 4.467ms | 10.987ms |


## File responses

Comparison between Granian application protocols using ~50KB JPEG image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

Tests are run with `--runtime-blocking-threads 1`.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c128) | 494402 | 49439 | 2.578ms | 60.546ms |
| ASGI (c128) | 288425 | 28847 | 4.425ms | 60.092ms |
| ASGI pathsend (c128) | 471775 | 47174 | 2.7ms | 77.695ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [AsyncIO-specific benchmarks](./asyncio.md)
- [Python versions](./pyver.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
