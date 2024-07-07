# Granian benchmarks



Run at: Sun 07 Jul 2024, 16:40    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.5.0    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c128) | 451837 | 45231 | 2.821ms | 25.756ms |
| str small (c128) | 411512 | 41191 | 3.097ms | 23.925ms |
| bytes big (c64) | 280418 | 28047 | 2.276ms | 5.666ms |
| str big (c64) | 287929 | 28796 | 2.217ms | 5.905ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c64) | 413211 | 41321 | 1.546ms | 3.964ms |
| RSGI str (c256) | 426394 | 42762 | 5.952ms | 77.356ms |
| RSGI echo (c512) | 375364 | 37760 | 13.47ms | 164.291ms |
| ASGI bytes (c512) | 422389 | 42441 | 12.0ms | 149.999ms |
| ASGI str (c512) | 421167 | 42302 | 12.039ms | 136.599ms |
| ASGI echo (c512) | 219785 | 22084 | 23.061ms | 108.419ms |
| WSGI bytes (c64) | 379838 | 37992 | 1.68ms | 4.02ms |
| WSGI str (c128) | 377965 | 37859 | 3.369ms | 18.501ms |
| WSGI echo (c64) | 350180 | 35024 | 1.823ms | 4.758ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c512) | 443970 | 44606 | 11.416ms | 153.453ms |
| HTTP/1 [POST] (c128) | 364571 | 36507 | 3.493ms | 23.512ms |
| HTTP/2 [GET] (c64) | 416862 | 41694 | 1.532ms | 6.128ms |
| HTTP/2 [POST] (c512) | 286831 | 28799 | 17.653ms | 255.309ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c64) | 347783 | 34788 | 1.833ms | 4.785ms |
| ASGI (c256) | 167350 | 16784 | 15.169ms | 69.669ms |
| ASGI pathsend (c64) | 299513 | 29954 | 2.132ms | 7.075ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
