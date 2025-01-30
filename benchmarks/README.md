# Granian benchmarks



Run at: Thu 30 Jan 2025, 02:28    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Python version: 3.11    
Granian version: 1.7.6

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c256) | 909712 | 91109 | 2.807ms | 15.337ms |
| str small (c256) | 901597 | 90315 | 2.832ms | 15.639ms |
| bytes big (c128) | 494866 | 49503 | 2.583ms | 14.92ms |
| str big (c128) | 445717 | 44596 | 2.867ms | 18.307ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c256) | 900062 | 90119 | 2.838ms | 12.855ms |
| RSGI str (c256) | 881674 | 88317 | 2.896ms | 15.554ms |
| RSGI echo (c256) | 627527 | 62848 | 4.068ms | 19.695ms |
| ASGI bytes (c128) | 748186 | 74841 | 1.709ms | 13.994ms |
| ASGI str (c128) | 750228 | 75069 | 1.703ms | 14.18ms |
| ASGI echo (c64) | 503022 | 50302 | 1.271ms | 2.575ms |
| WSGI bytes (c256) | 642563 | 64367 | 3.973ms | 15.454ms |
| WSGI str (c64) | 629022 | 62904 | 1.016ms | 2.361ms |
| WSGI echo (c256) | 581620 | 58241 | 4.391ms | 13.409ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c256) | 897954 | 89939 | 2.843ms | 14.539ms |
| HTTP/1 [POST] (c128) | 647934 | 64831 | 1.972ms | 12.859ms |
| HTTP/2 [GET] (c256) | 874234 | 87629 | 2.918ms | 46.195ms |
| HTTP/2 [POST] (c128) | 634648 | 63508 | 2.012ms | 12.346ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c256) | 748074 | 74903 | 3.411ms | 25.448ms |
| ASGI (c64) | 328688 | 32870 | 1.945ms | 2.515ms |
| ASGI pathsend (c512) | 631845 | 63485 | 8.052ms | 69.429ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
- [Python versions](./pyver.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
