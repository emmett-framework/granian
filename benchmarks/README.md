# Granian benchmarks



Run at: Mon 15 Apr 2024, 18:43    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.2.3    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c256) | 670513 | 44816 | 5.696ms | 72.554ms |
| str small (c64) | 626602 | 41777 | 1.529ms | 4.013ms |
| bytes big (c32) | 405320 | 27021 | 1.184ms | 3.515ms |
| str big (c64) | 390094 | 26009 | 2.457ms | 6.804ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c128) | 643993 | 42963 | 2.971ms | 36.89ms |
| RSGI str (c64) | 639816 | 42660 | 1.497ms | 4.549ms |
| RSGI echo (c256) | 424305 | 28349 | 9.002ms | 58.785ms |
| ASGI bytes (c256) | 589976 | 39393 | 6.478ms | 56.332ms |
| ASGI str (c128) | 595539 | 39727 | 3.212ms | 41.071ms |
| ASGI echo (c256) | 358026 | 23916 | 10.67ms | 70.183ms |
| WSGI bytes (c128) | 601099 | 40113 | 3.183ms | 20.743ms |
| WSGI str (c64) | 594859 | 39662 | 1.611ms | 4.107ms |
| WSGI echo (c64) | 536977 | 35804 | 1.784ms | 4.89ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c64) | 656240 | 43754 | 1.46ms | 5.256ms |
| HTTP/1 [POST] (c128) | 411477 | 27451 | 4.653ms | 29.332ms |
| HTTP/2 [GET] (c64) | 619226 | 41285 | 1.548ms | 6.782ms |
| HTTP/2 [POST] (c64) | 384420 | 25631 | 2.493ms | 5.795ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c128) | 524859 | 35012 | 3.648ms | 29.12ms |
| ASGI (c128) | 230771 | 15395 | 8.292ms | 33.273ms |
| ASGI pathsend (c128) | 273647 | 18256 | 6.991ms | 36.1ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
