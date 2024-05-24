# Granian benchmarks



Run at: Fri 24 May 2024, 14:32    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.3.2    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c128) | 701183 | 46775 | 2.728ms | 41.048ms |
| str small (c128) | 639563 | 42666 | 2.992ms | 36.68ms |
| bytes big (c32) | 413777 | 27585 | 1.159ms | 3.17ms |
| str big (c64) | 418222 | 27885 | 2.291ms | 6.227ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c128) | 684648 | 45695 | 2.793ms | 27.961ms |
| RSGI str (c64) | 658914 | 43931 | 1.454ms | 4.18ms |
| RSGI echo (c256) | 436417 | 29140 | 8.76ms | 64.167ms |
| ASGI bytes (c128) | 618815 | 41290 | 3.093ms | 20.697ms |
| ASGI str (c32) | 606968 | 40464 | 0.79ms | 2.889ms |
| ASGI echo (c128) | 387116 | 25827 | 4.943ms | 34.383ms |
| WSGI bytes (c128) | 611036 | 40770 | 3.132ms | 28.936ms |
| WSGI str (c128) | 597466 | 39867 | 3.2ms | 40.65ms |
| WSGI echo (c64) | 556209 | 37085 | 1.723ms | 4.702ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c32) | 670247 | 44682 | 0.716ms | 1.786ms |
| HTTP/1 [POST] (c256) | 434632 | 29031 | 8.792ms | 56.178ms |
| HTTP/2 [GET] (c64) | 623568 | 41577 | 1.537ms | 4.72ms |
| HTTP/2 [POST] (c64) | 397725 | 26520 | 2.409ms | 6.884ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c256) | 525916 | 35133 | 7.267ms | 60.284ms |
| ASGI (c128) | 244694 | 16324 | 7.822ms | 26.267ms |
| ASGI pathsend (c128) | 436928 | 29152 | 4.375ms | 43.711ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
