# Granian benchmarks



Run at: Sun 26 May 2024, 08:48    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.4.0    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c128) | 645136 | 43047 | 2.964ms | 35.913ms |
| str small (c256) | 682896 | 45605 | 5.598ms | 53.379ms |
| bytes big (c64) | 414907 | 27665 | 2.309ms | 5.729ms |
| str big (c64) | 428762 | 28589 | 2.233ms | 6.921ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c64) | 671256 | 44755 | 1.428ms | 3.987ms |
| RSGI str (c64) | 669807 | 44659 | 1.43ms | 4.906ms |
| RSGI echo (c64) | 612353 | 40829 | 1.564ms | 4.402ms |
| ASGI bytes (c512) | 618058 | 41359 | 12.327ms | 135.647ms |
| ASGI str (c512) | 605561 | 40557 | 12.585ms | 123.101ms |
| ASGI echo (c64) | 366514 | 24436 | 2.615ms | 5.406ms |
| WSGI bytes (c256) | 604499 | 40372 | 6.324ms | 48.487ms |
| WSGI str (c512) | 602714 | 40347 | 12.635ms | 166.804ms |
| WSGI echo (c128) | 534302 | 35648 | 3.581ms | 27.538ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c256) | 631914 | 42206 | 6.048ms | 63.354ms |
| HTTP/1 [POST] (c512) | 616195 | 41238 | 12.378ms | 116.836ms |
| HTTP/2 [GET] (c64) | 636209 | 42419 | 1.507ms | 4.572ms |
| HTTP/2 [POST] (c128) | 481880 | 32148 | 3.971ms | 32.601ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c64) | 470616 | 31381 | 2.034ms | 5.498ms |
| ASGI (c512) | 239884 | 16054 | 31.745ms | 132.05ms |
| ASGI pathsend (c64) | 401796 | 26789 | 2.384ms | 6.176ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
