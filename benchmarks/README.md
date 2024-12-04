# Granian benchmarks



Run at: Wed 04 Dec 2024, 18:15    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.7.0    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c256) | 351317 | 35212 | 7.238ms | 57.561ms |
| str small (c256) | 346475 | 34745 | 7.327ms | 77.759ms |
| bytes big (c128) | 241554 | 24181 | 5.27ms | 36.572ms |
| str big (c128) | 237907 | 23813 | 5.355ms | 38.575ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c256) | 347040 | 34828 | 7.313ms | 73.329ms |
| RSGI str (c512) | 349522 | 35148 | 14.485ms | 157.147ms |
| RSGI echo (c512) | 213607 | 21484 | 23.703ms | 114.999ms |
| ASGI bytes (c256) | 294195 | 29503 | 8.637ms | 53.795ms |
| ASGI str (c512) | 296628 | 29823 | 17.07ms | 130.156ms |
| ASGI echo (c256) | 193724 | 19426 | 13.109ms | 64.891ms |
| WSGI bytes (c64) | 381867 | 38193 | 1.672ms | 4.296ms |
| WSGI str (c64) | 377812 | 37787 | 1.689ms | 4.804ms |
| WSGI echo (c64) | 379539 | 37960 | 1.683ms | 4.588ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c512) | 345568 | 34702 | 14.679ms | 124.351ms |
| HTTP/1 [POST] (c512) | 210922 | 21198 | 23.999ms | 105.032ms |
| HTTP/2 [GET] (c256) | 310008 | 31092 | 8.191ms | 89.156ms |
| HTTP/2 [POST] (c256) | 197626 | 19825 | 12.841ms | 93.535ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c512) | 273552 | 27536 | 18.467ms | 123.024ms |
| ASGI (c64) | 137930 | 13796 | 4.626ms | 7.065ms |
| ASGI pathsend (c512) | 248181 | 24967 | 20.36ms | 116.83ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
- [Python versions](./pyver.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
