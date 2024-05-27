# Granian benchmarks



Run at: Mon 27 May 2024, 06:54    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.4.0    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c512) | 621740 | 41600 | 12.263ms | 152.217ms |
| str small (c64) | 681620 | 45447 | 1.406ms | 4.134ms |
| bytes big (c64) | 422346 | 28158 | 2.269ms | 6.492ms |
| str big (c64) | 431171 | 28749 | 2.221ms | 6.265ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c64) | 676033 | 45074 | 1.417ms | 4.416ms |
| RSGI str (c64) | 658183 | 43882 | 1.456ms | 4.514ms |
| RSGI echo (c256) | 618784 | 41351 | 6.174ms | 78.008ms |
| ASGI bytes (c64) | 661880 | 44129 | 1.448ms | 4.237ms |
| ASGI str (c64) | 649292 | 43291 | 1.476ms | 4.393ms |
| ASGI echo (c128) | 368270 | 24568 | 5.198ms | 24.292ms |
| WSGI bytes (c64) | 601224 | 40086 | 1.594ms | 4.342ms |
| WSGI str (c64) | 591918 | 39469 | 1.617ms | 3.864ms |
| WSGI echo (c256) | 543488 | 36308 | 7.03ms | 74.972ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c128) | 663758 | 44276 | 2.882ms | 41.407ms |
| HTTP/1 [POST] (c512) | 618784 | 41414 | 12.319ms | 153.226ms |
| HTTP/2 [GET] (c128) | 623845 | 41629 | 3.068ms | 22.093ms |
| HTTP/2 [POST] (c256) | 466859 | 31195 | 8.179ms | 107.051ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c512) | 509384 | 34082 | 14.956ms | 109.014ms |
| ASGI (c128) | 245421 | 16374 | 7.8ms | 19.693ms |
| ASGI pathsend (c64) | 442263 | 29489 | 2.166ms | 5.212ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
