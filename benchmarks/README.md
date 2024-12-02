# Granian benchmarks



Run at: Sun 01 Dec 2024, 23:56    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.7.0    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c512) | 346688 | 34872 | 14.601ms | 162.626ms |
| str small (c512) | 360279 | 36234 | 14.055ms | 117.1ms |
| bytes big (c128) | 236368 | 23681 | 5.383ms | 32.602ms |
| str big (c128) | 239789 | 23997 | 5.315ms | 32.382ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c512) | 354716 | 35609 | 14.303ms | 144.293ms |
| RSGI str (c512) | 348502 | 35012 | 14.549ms | 122.899ms |
| RSGI echo (c256) | 203084 | 20396 | 12.478ms | 81.425ms |
| ASGI bytes (c256) | 294820 | 29558 | 8.618ms | 70.806ms |
| ASGI str (c256) | 292409 | 29333 | 8.695ms | 67.481ms |
| ASGI echo (c512) | 182131 | 18295 | 27.801ms | 106.883ms |
| WSGI bytes (c64) | 380393 | 38049 | 1.677ms | 4.464ms |
| WSGI str (c256) | 375259 | 37652 | 6.762ms | 82.201ms |
| WSGI echo (c64) | 353336 | 35338 | 1.807ms | 5.51ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c512) | 369299 | 37122 | 13.732ms | 109.354ms |
| HTTP/1 [POST] (c128) | 204831 | 20514 | 6.213ms | 32.318ms |
| HTTP/2 [GET] (c512) | 308078 | 30935 | 16.438ms | 175.483ms |
| HTTP/2 [POST] (c256) | 184626 | 18523 | 13.739ms | 82.064ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c512) | 273644 | 27517 | 18.492ms | 114.065ms |
| ASGI (c128) | 136406 | 13654 | 9.336ms | 36.727ms |
| ASGI pathsend (c256) | 245210 | 24580 | 10.367ms | 56.706ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
- [Python versions](./pyver.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
