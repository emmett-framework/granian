# Granian benchmarks



Run at: Sat 27 Apr 2024, 01:11    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.3.0    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c128) | 685555 | 45754 | 2.79ms | 32.277ms |
| str small (c128) | 694975 | 46357 | 2.755ms | 25.382ms |
| bytes big (c32) | 408297 | 27219 | 1.175ms | 3.63ms |
| str big (c64) | 410433 | 27362 | 2.336ms | 6.35ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c64) | 661679 | 44117 | 1.448ms | 4.51ms |
| RSGI str (c64) | 682202 | 45483 | 1.405ms | 3.903ms |
| RSGI echo (c256) | 435515 | 29107 | 8.765ms | 76.847ms |
| ASGI bytes (c128) | 597651 | 39892 | 3.199ms | 27.78ms |
| ASGI str (c128) | 609985 | 40697 | 3.136ms | 32.221ms |
| ASGI echo (c64) | 378864 | 25262 | 2.527ms | 5.252ms |
| WSGI bytes (c256) | 601162 | 40128 | 6.361ms | 57.809ms |
| WSGI str (c64) | 597423 | 39833 | 1.604ms | 3.871ms |
| WSGI echo (c64) | 564423 | 37637 | 1.696ms | 4.475ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c64) | 685626 | 45712 | 1.397ms | 4.258ms |
| HTTP/1 [POST] (c256) | 433246 | 28952 | 8.808ms | 77.372ms |
| HTTP/2 [GET] (c64) | 626282 | 41759 | 1.53ms | 7.665ms |
| HTTP/2 [POST] (c64) | 398394 | 26565 | 2.404ms | 5.517ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c256) | 522222 | 34881 | 7.316ms | 64.539ms |
| ASGI (c256) | 242608 | 16196 | 15.749ms | 62.065ms |
| ASGI pathsend (c256) | 283263 | 18917 | 13.486ms | 56.627ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
