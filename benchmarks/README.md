# Granian benchmarks



Run at: Thu 10 Apr 2025, 17:04    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Python version: 3.12    
Granian version: 2.2.4

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes 10B (c256) | 784930 | 78662 | 3.249ms | 46.739ms |
| str 10B (c512) | 790973 | 79502 | 6.43ms | 38.533ms |
| bytes 100KB (c128) | 564015 | 56422 | 2.264ms | 27.279ms |
| str 100KB (c128) | 563350 | 56378 | 2.267ms | 30.896ms |


## Interfaces

Comparison between Granian application protocols using plain text responses.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI get 1KB (c256) | 785847 | 78821 | 3.243ms | 26.172ms |
| RSGI echo 1KB (c256) | 498052 | 49910 | 5.111ms | 42.401ms |
| RSGI echo 100KB (iter) (c128) | 170218 | 17047 | 7.482ms | 31.619ms |
| ASGI get 1KB (c128) | 619821 | 62039 | 2.06ms | 29.523ms |
| ASGI echo 1KB (c128) | 443863 | 44412 | 2.874ms | 22.326ms |
| ASGI echo 100KB (iter) (c256) | 170081 | 17048 | 14.956ms | 81.81ms |
| WSGI get 1KB (c512) | 645205 | 64854 | 7.883ms | 67.916ms |
| WSGI echo 1KB (c64) | 577620 | 57769 | 1.105ms | 2.289ms |
| WSGI echo 100KB (iter) (c64) | 69523 | 6952 | 9.188ms | 29.052ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using plain text responses.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 get 1KB (c256) | 785134 | 78694 | 3.249ms | 21.538ms |
| HTTP/1 echo 1KB (c128) | 564695 | 56516 | 2.257ms | 29.013ms |
| HTTP/2 get 1KB (c256) | 698309 | 70061 | 3.649ms | 53.787ms |
| HTTP/2 echo 1KB (c512) | 123215 | 12382 | 41.178ms | 118.765ms |


## File responses

Comparison between Granian application protocols using ~50KB image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c128) | 118083 | 11812 | 10.811ms | 20.669ms |
| ASGI (c128) | 221812 | 22194 | 5.759ms | 28.457ms |
| ASGI pathsend (c512) | 298402 | 30015 | 16.972ms | 148.133ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [AsyncIO-specific benchmarks](./asyncio.md)
- [Python versions](./pyver.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
