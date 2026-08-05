# Granian benchmarks



Run at: Wed 05 Aug 2026, 16:42    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.18.41 (CPUs: 16)    
Python version: 3.13    
Granian version: 2.8.1

### Methodology

Unless otherwise specified in the specific benchmark section, Granian is run:

- Using default configuration, thus:
  - 1 worker
  - 1 runtime thread
- With `--runtime-mode` set to `auto`
- With `--http 1` flag
- With `--no-ws` flag
- With `uvloop` event-loop on async protocols

Tests are peformed using `oha` utility, with the concurrency specified in the specific test. The test run for 10 seconds, preceeded by a *primer* run at concurrency 8 for 4 seconds, and a *warmup* run at the maximum configured concurrency for the test for 3 seconds.

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes 10B (c64) | 1464838 | 146427 | 0.435ms | 39.753ms |
| str 10B (c64) | 1449800 | 144931 | 0.439ms | 36.381ms |
| bytes 100KB (c64) | 562072 | 56194 | 1.135ms | 36.921ms |
| str 100KB (c64) | 572818 | 57275 | 1.113ms | 36.285ms |


## Interfaces

Comparison between Granian application protocols using plain text responses.

The 1KB GET and POST tests are run with `--blocking-threads 1`.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI get 1KB (c128) | 1430994 | 143073 | 0.891ms | 49.009ms |
| RSGI echo 1KB (c128) | 1199952 | 119975 | 1.062ms | 71.355ms |
| RSGI echo 100KB (iter) (c64) | 176984 | 17699 | 3.605ms | 45.524ms |
| ASGI get 1KB (c128) | 1408587 | 140814 | 0.906ms | 36.76ms |
| ASGI echo 1KB (c128) | 1024809 | 102467 | 1.246ms | 34.832ms |
| ASGI echo 100KB (iter) (c64) | 183551 | 18359 | 3.475ms | 49.857ms |
| WSGI get 1KB (c64) | 1439918 | 143939 | 0.443ms | 22.739ms |
| WSGI echo 1KB (c64) | 1390024 | 138958 | 0.458ms | 33.443ms |
| WSGI echo 100KB (iter) (c64) | 100679 | 10072 | 6.344ms | 36.668ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using plain text responses.

All tests are run with `--runtime-threads 2`.
HTTP/2 tests are run with `--http 2`.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 get 1KB (c128) | 2382173 | 238104 | 0.535ms | 43.713ms |
| HTTP/1 echo 1KB (c128) | 1583227 | 158278 | 0.805ms | 44.055ms |
| HTTP/2 get 1KB (c128) | 2000276 | 199991 | 2.545ms | 9.009ms |
| HTTP/2 echo 1KB (c128) | 1431555 | 143141 | 3.557ms | 10.351ms |


## File responses

Comparison between Granian application protocols using ~50KB JPEG image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

Tests are run with `--runtime-blocking-threads 1`.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c128) | 483291 | 48329 | 2.64ms | 68.305ms |
| ASGI (c128) | 383219 | 38321 | 3.329ms | 51.698ms |
| ASGI pathsend (c128) | 481069 | 48108 | 2.65ms | 74.848ms |


### Other benchmarks

- [Concurrency benchmarks](./concurrency.md)
- [Versus 3rd party servers](./vs.md)
- [AsyncIO-specific benchmarks](./asyncio.md)
- [Python versions](./pyver.md)
