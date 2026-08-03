# Granian benchmarks



Run at: Mon 03 Aug 2026, 12:37    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.18.41 (CPUs: 16)    
Python version: 3.13    
Granian version: 2.8.0

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
| bytes 10B (c64) | 1478898 | 147834 | 0.431ms | 29.2ms |
| str 10B (c64) | 1470102 | 146951 | 0.433ms | 35.542ms |
| bytes 100KB (c64) | 565924 | 56579 | 1.127ms | 25.056ms |
| str 100KB (c64) | 572614 | 57251 | 1.113ms | 38.346ms |


## Interfaces

Comparison between Granian application protocols using plain text responses.

The 1KB GET and POST tests are run with `--blocking-threads 1`.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI get 1KB (c128) | 1440061 | 143968 | 0.885ms | 55.875ms |
| RSGI echo 1KB (c128) | 1253848 | 125350 | 1.017ms | 53.773ms |
| RSGI echo 100KB (iter) (c64) | 170504 | 17054 | 3.744ms | 36.51ms |
| ASGI get 1KB (c128) | 1345988 | 134571 | 0.947ms | 67.815ms |
| ASGI echo 1KB (c128) | 906887 | 90672 | 1.406ms | 44.779ms |
| ASGI echo 100KB (iter) (c64) | 190543 | 19056 | 3.35ms | 35.072ms |
| WSGI get 1KB (c64) | 1455635 | 145517 | 0.438ms | 35.965ms |
| WSGI echo 1KB (c64) | 1377045 | 137662 | 0.463ms | 34.602ms |
| WSGI echo 100KB (iter) (c64) | 100855 | 10090 | 6.332ms | 34.801ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using plain text responses.

All tests are run with `--runtime-threads 2`.
HTTP/2 tests are run with `--http 2`.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 get 1KB (c128) | 2434204 | 243333 | 0.524ms | 36.302ms |
| HTTP/1 echo 1KB (c128) | 1572323 | 157196 | 0.811ms | 44.33ms |
| HTTP/2 get 1KB (c128) | 2039868 | 203943 | 2.497ms | 10.459ms |
| HTTP/2 echo 1KB (c128) | 1400511 | 140020 | 3.634ms | 10.965ms |


## File responses

Comparison between Granian application protocols using ~50KB JPEG image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

Tests are run with `--runtime-blocking-threads 1`.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c128) | 471413 | 47139 | 2.705ms | 58.164ms |
| ASGI (c128) | 371143 | 37108 | 3.438ms | 48.285ms |
| ASGI pathsend (c128) | 479851 | 47983 | 2.657ms | 59.238ms |


### Other benchmarks

- [Concurrency benchmarks](./concurrency.md)
- [Versus 3rd party servers](./vs.md)
- [AsyncIO-specific benchmarks](./asyncio.md)
- [Python versions](./pyver.md)
