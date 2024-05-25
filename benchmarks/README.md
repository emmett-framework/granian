# Granian benchmarks



Run at: Sat 25 May 2024, 14:50    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.3.2    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c64) | 658003 | 43872 | 1.456ms | 3.792ms |
| str small (c64) | 614924 | 40999 | 1.559ms | 8.028ms |
| bytes big (c64) | 424921 | 28332 | 2.255ms | 6.356ms |
| str big (c64) | 428458 | 28569 | 2.235ms | 6.073ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c512) | 650267 | 43511 | 11.724ms | 156.874ms |
| RSGI str (c64) | 636643 | 42445 | 1.505ms | 3.953ms |
| RSGI echo (c512) | 610597 | 40859 | 12.485ms | 161.427ms |
| ASGI bytes (c64) | 646278 | 43090 | 1.482ms | 4.199ms |
| ASGI str (c512) | 598474 | 40011 | 12.752ms | 136.724ms |
| ASGI echo (c64) | 370963 | 24733 | 2.583ms | 5.207ms |
| WSGI bytes (c64) | 249135 | 16611 | 3.845ms | 57.358ms |
| WSGI str (c64) | 258734 | 17250 | 3.703ms | 40.239ms |
| WSGI echo (c64) | 203655 | 13579 | 4.702ms | 58.418ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c128) | 684486 | 45660 | 2.797ms | 28.217ms |
| HTTP/1 [POST] (c128) | 613247 | 40912 | 3.12ms | 31.946ms |
| HTTP/2 [GET] (c64) | 623058 | 41542 | 1.538ms | 4.881ms |
| HTTP/2 [POST] (c128) | 467232 | 31170 | 4.096ms | 39.885ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c128) | 400732 | 26733 | 4.778ms | 24.403ms |
| ASGI (c512) | 241600 | 16172 | 31.503ms | 132.476ms |
| ASGI pathsend (c512) | 397554 | 26591 | 19.182ms | 132.831ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
