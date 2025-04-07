# Granian benchmarks



Run at: Mon 07 Apr 2025, 11:16    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Python version: 3.12    
Granian version: 2.2.2

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes 10B (c256) | 806925 | 80834 | 3.163ms | 20.017ms |
| str 10B (c256) | 804130 | 80658 | 3.167ms | 23.669ms |
| bytes 100KB (c128) | 560652 | 56089 | 2.277ms | 23.43ms |
| str 100KB (c128) | 561556 | 56194 | 2.275ms | 21.173ms |


## Interfaces

Comparison between Granian application protocols using plain text responses.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI get 1KB (c256) | 799584 | 80187 | 3.18ms | 47.318ms |
| RSGI echo 1KB (c128) | 584744 | 58493 | 2.184ms | 15.204ms |
| RSGI echo 100KB (iter) (c64) | 221484 | 22150 | 2.881ms | 10.076ms |
| ASGI get 1KB (c128) | 631672 | 63234 | 2.021ms | 28.119ms |
| ASGI echo 1KB (c128) | 442589 | 44308 | 2.885ms | 27.757ms |
| ASGI echo 100KB (iter) (c64) | 223092 | 22308 | 2.864ms | 9.759ms |
| WSGI get 1KB (c64) | 641386 | 64139 | 0.997ms | 1.851ms |
| WSGI echo 1KB (c128) | 585696 | 58609 | 2.182ms | 9.507ms |
| WSGI echo 100KB (iter) (c64) | 87939 | 8794 | 7.26ms | 25.514ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using plain text responses.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 get 1KB (c256) | 792805 | 79408 | 3.22ms | 23.79ms |
| HTTP/1 echo 1KB (c128) | 569464 | 57017 | 2.237ms | 21.665ms |
| HTTP/2 get 1KB (c256) | 692572 | 69408 | 3.684ms | 61.474ms |
| HTTP/2 echo 1KB (c512) | 123226 | 12391 | 41.168ms | 111.67ms |


## File responses

Comparison between Granian application protocols using ~50KB image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c128) | 117451 | 11759 | 10.862ms | 31.353ms |
| ASGI (c128) | 220389 | 22070 | 5.789ms | 32.804ms |
| ASGI pathsend (c512) | 293847 | 29509 | 17.279ms | 127.314ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [AsyncIO-specific benchmarks](./asyncio.md)
- [Python versions](./pyver.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
