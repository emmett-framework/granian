# Granian benchmarks



Run at: Mon 28 Oct 2024, 02:09    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.6.2    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c256) | 392764 | 39392 | 6.47ms | 61.121ms |
| str small (c512) | 424544 | 42723 | 11.921ms | 130.529ms |
| bytes big (c64) | 288745 | 28874 | 2.212ms | 6.843ms |
| str big (c64) | 276754 | 27681 | 2.305ms | 5.861ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c64) | 397815 | 39792 | 1.602ms | 4.506ms |
| RSGI str (c64) | 414448 | 41451 | 1.541ms | 4.152ms |
| RSGI echo (c256) | 366975 | 36780 | 6.93ms | 59.373ms |
| ASGI bytes (c512) | 394155 | 39596 | 12.855ms | 143.895ms |
| ASGI str (c64) | 402564 | 40258 | 1.585ms | 3.888ms |
| ASGI echo (c128) | 217107 | 21742 | 5.864ms | 31.625ms |
| WSGI bytes (c128) | 375703 | 37613 | 3.386ms | 39.83ms |
| WSGI str (c256) | 371867 | 37268 | 6.837ms | 58.755ms |
| WSGI echo (c128) | 352872 | 35325 | 3.609ms | 33.688ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c64) | 402839 | 40293 | 1.584ms | 3.983ms |
| HTTP/1 [POST] (c64) | 361471 | 36152 | 1.766ms | 5.997ms |
| HTTP/2 [GET] (c128) | 411331 | 41181 | 3.097ms | 35.375ms |
| HTTP/2 [POST] (c256) | 285962 | 28684 | 8.881ms | 102.249ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c64) | 336910 | 33698 | 1.894ms | 9.23ms |
| ASGI (c64) | 167204 | 16723 | 3.818ms | 5.76ms |
| ASGI pathsend (c128) | 290053 | 29038 | 4.392ms | 28.584ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
- [Python versions](./pyver.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
