# Granian benchmarks



Run at: Mon 02 Dec 2024, 00:49    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.7.0    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c512) | 350924 | 35320 | 14.393ms | 138.191ms |
| str small (c512) | 353008 | 35476 | 14.348ms | 139.591ms |
| bytes big (c128) | 226720 | 22696 | 5.621ms | 25.127ms |
| str big (c128) | 219153 | 21938 | 5.815ms | 30.467ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c512) | 372070 | 37401 | 13.609ms | 137.038ms |
| RSGI str (c512) | 353607 | 35499 | 14.351ms | 136.677ms |
| RSGI echo (c64) | 246863 | 24691 | 2.586ms | 4.727ms |
| ASGI bytes (c256) | 291050 | 29187 | 8.728ms | 70.547ms |
| ASGI str (c256) | 287503 | 28821 | 8.84ms | 64.649ms |
| ASGI echo (c64) | 185088 | 18512 | 3.447ms | 6.956ms |
| WSGI bytes (c64) | 375895 | 37600 | 1.697ms | 4.943ms |
| WSGI str (c64) | 381233 | 38132 | 1.673ms | 4.388ms |
| WSGI echo (c64) | 357818 | 35787 | 1.784ms | 4.601ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c512) | 348972 | 35077 | 14.517ms | 147.058ms |
| HTTP/1 [POST] (c64) | 250115 | 25017 | 2.551ms | 4.383ms |
| HTTP/2 [GET] (c128) | 298460 | 29890 | 4.259ms | 44.696ms |
| HTTP/2 [POST] (c128) | 243728 | 24402 | 5.223ms | 37.367ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c128) | 264408 | 26467 | 4.818ms | 27.746ms |
| ASGI (c128) | 134844 | 13501 | 9.439ms | 28.357ms |
| ASGI pathsend (c256) | 237813 | 23863 | 10.686ms | 60.256ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
- [Python versions](./pyver.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
