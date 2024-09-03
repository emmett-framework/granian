# Granian benchmarks



Run at: Tue 03 Sep 2024, 21:44    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.6.0    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c64) | 413639 | 41365 | 1.544ms | 4.612ms |
| str small (c128) | 433016 | 43346 | 2.944ms | 19.673ms |
| bytes big (c64) | 281576 | 28162 | 2.267ms | 5.781ms |
| str big (c64) | 288430 | 28851 | 2.211ms | 6.574ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c128) | 423456 | 42379 | 3.009ms | 27.917ms |
| RSGI str (c128) | 428954 | 42942 | 2.972ms | 24.262ms |
| RSGI echo (c256) | 377133 | 37812 | 6.735ms | 74.44ms |
| ASGI bytes (c64) | 436301 | 43632 | 1.463ms | 4.525ms |
| ASGI str (c64) | 422612 | 42269 | 1.51ms | 4.182ms |
| ASGI echo (c512) | 217386 | 21864 | 23.309ms | 121.784ms |
| WSGI bytes (c128) | 367719 | 36827 | 3.463ms | 18.241ms |
| WSGI str (c64) | 373343 | 37340 | 1.71ms | 4.425ms |
| WSGI echo (c64) | 342874 | 34293 | 1.861ms | 4.515ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c128) | 409169 | 40964 | 3.11ms | 35.937ms |
| HTTP/1 [POST] (c256) | 367186 | 36816 | 6.927ms | 63.568ms |
| HTTP/2 [GET] (c64) | 429019 | 42908 | 1.489ms | 6.358ms |
| HTTP/2 [POST] (c64) | 279182 | 27921 | 2.287ms | 6.201ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c64) | 340095 | 34014 | 1.877ms | 3.9ms |
| ASGI (c256) | 168845 | 16957 | 14.998ms | 81.803ms |
| ASGI pathsend (c64) | 299960 | 29998 | 2.129ms | 4.663ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
