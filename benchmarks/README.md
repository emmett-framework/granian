# Granian benchmarks



Run at: Fri 11 Apr 2025, 12:24    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Python version: 3.12    
Granian version: 2.2.4

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes 10B (c256) | 1296160 | 129939 | 1.968ms | 25.707ms |
| str 10B (c128) | 1286985 | 128813 | 0.992ms | 24.702ms |
| bytes 100KB (c64) | 590156 | 59023 | 1.082ms | 5.519ms |
| str 100KB (c64) | 606544 | 60658 | 1.053ms | 5.121ms |


## Interfaces

Comparison between Granian application protocols using plain text responses.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI get 1KB (c64) | 1309828 | 130983 | 0.488ms | 1.719ms |
| RSGI echo 1KB (c64) | 1234311 | 123429 | 0.518ms | 1.614ms |
| RSGI echo 100KB (iter) (c128) | 199128 | 19934 | 6.397ms | 40.525ms |
| ASGI get 1KB (c256) | 1272020 | 127659 | 2.003ms | 26.942ms |
| ASGI echo 1KB (c128) | 930783 | 93132 | 1.372ms | 9.704ms |
| ASGI echo 100KB (iter) (c64) | 274993 | 27499 | 2.323ms | 9.679ms |
| WSGI get 1KB (c64) | 1304309 | 130428 | 0.49ms | 2.678ms |
| WSGI echo 1KB (c64) | 1229152 | 122920 | 0.52ms | 1.595ms |
| WSGI echo 100KB (iter) (c64) | 90983 | 9099 | 7.021ms | 21.154ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using plain text responses.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 get 1KB (c64) | 1274438 | 127460 | 0.501ms | 1.67ms |
| HTTP/1 echo 1KB (c64) | 1227054 | 122715 | 0.52ms | 2.372ms |
| HTTP/2 get 1KB (c256) | 742970 | 74375 | 3.431ms | 96.049ms |
| HTTP/2 echo 1KB (c512) | 123651 | 12415 | 41.058ms | 113.348ms |


## File responses

Comparison between Granian application protocols using ~50KB image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c128) | 512445 | 51267 | 2.494ms | 28.725ms |
| ASGI (c64) | 299918 | 29994 | 2.13ms | 3.825ms |
| ASGI pathsend (c128) | 497969 | 49836 | 2.563ms | 32.08ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [AsyncIO-specific benchmarks](./asyncio.md)
- [Python versions](./pyver.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
