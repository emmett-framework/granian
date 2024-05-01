# Granian benchmarks



Run at: Wed 01 May 2024, 21:13    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.3.1    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c64) | 626131 | 41747 | 1.53ms | 3.976ms |
| str small (c256) | 656342 | 43853 | 5.814ms | 81.793ms |
| bytes big (c32) | 414209 | 27614 | 1.158ms | 3.596ms |
| str big (c64) | 406256 | 27087 | 2.358ms | 6.279ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c128) | 636768 | 42481 | 3.006ms | 27.87ms |
| RSGI str (c64) | 654773 | 43655 | 1.463ms | 3.738ms |
| RSGI echo (c256) | 428875 | 28644 | 8.908ms | 76.577ms |
| ASGI bytes (c128) | 594226 | 39645 | 3.221ms | 28.739ms |
| ASGI str (c64) | 608232 | 40553 | 1.575ms | 3.997ms |
| ASGI echo (c128) | 389133 | 25960 | 4.919ms | 32.617ms |
| WSGI bytes (c32) | 602109 | 40138 | 0.796ms | 2.494ms |
| WSGI str (c128) | 638006 | 42570 | 2.998ms | 27.818ms |
| WSGI echo (c64) | 539308 | 35960 | 1.776ms | 5.169ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c64) | 661064 | 44073 | 1.45ms | 3.493ms |
| HTTP/1 [POST] (c256) | 431154 | 28788 | 8.867ms | 60.043ms |
| HTTP/2 [GET] (c128) | 614523 | 41004 | 3.115ms | 22.503ms |
| HTTP/2 [POST] (c64) | 393047 | 26207 | 2.438ms | 7.256ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c128) | 513965 | 34288 | 3.725ms | 23.066ms |
| ASGI (c64) | 248384 | 16562 | 3.855ms | 5.662ms |
| ASGI pathsend (c128) | 433308 | 28908 | 4.418ms | 19.855ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
