# Granian benchmarks



Run at: Tue 07 Apr 2026, 11:25    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.12.77 (CPUs: 16)    
Python version: 3.13    
Granian version: 2.7.3

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
| bytes 10B (c64) | 1450961 | 145033 | 0.439ms | 25.405ms |
| str 10B (c64) | 1480230 | 147959 | 0.43ms | 52.436ms |
| bytes 100KB (c64) | 593398 | 59331 | 1.074ms | 52.242ms |
| str 100KB (c64) | 594070 | 59394 | 1.073ms | 42.795ms |


## Interfaces

Comparison between Granian application protocols using plain text responses.

The 1KB GET and POST tests are run with `--blocking-threads 1`.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI get 1KB (c128) | 1421433 | 142099 | 0.896ms | 87.046ms |
| RSGI echo 1KB (c128) | 1246244 | 124588 | 1.022ms | 76.837ms |
| RSGI echo 100KB (iter) (c64) | 171420 | 17143 | 3.723ms | 41.503ms |
| ASGI get 1KB (c128) | 1393345 | 139302 | 0.916ms | 56.471ms |
| ASGI echo 1KB (c128) | 1010462 | 101035 | 1.263ms | 57.267ms |
| ASGI echo 100KB (iter) (c64) | 194048 | 19404 | 3.289ms | 44.404ms |
| WSGI get 1KB (c64) | 1465008 | 146454 | 0.435ms | 30.114ms |
| WSGI echo 1KB (c64) | 1361498 | 136094 | 0.468ms | 37.591ms |
| WSGI echo 100KB (iter) (c64) | 109622 | 10966 | 5.824ms | 37.476ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using plain text responses.

All tests are run with `--runtime-threads 2`.
HTTP/2 tests are run with `--http 2`.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 get 1KB (c128) | 2481531 | 248039 | 0.513ms | 71.339ms |
| HTTP/1 echo 1KB (c128) | 1672558 | 167208 | 0.761ms | 66.556ms |
| HTTP/2 get 1KB (c128) | 2045952 | 204531 | 2.483ms | 10.003ms |
| HTTP/2 echo 1KB (c128) | 1483399 | 148302 | 3.421ms | 8.178ms |


## File responses

Comparison between Granian application protocols using ~50KB JPEG image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

Tests are run with `--runtime-blocking-threads 1`.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c128) | 483076 | 48308 | 2.642ms | 56.197ms |
| ASGI (c128) | 378697 | 37874 | 3.366ms | 69.31ms |
| ASGI pathsend (c128) | 477448 | 47741 | 2.672ms | 60.371ms |


### Other benchmarks

- [Concurrency benchmarks](./concurrency.md)
- [Versus 3rd party servers](./vs.md)
- [AsyncIO-specific benchmarks](./asyncio.md)
- [Python versions](./pyver.md)
