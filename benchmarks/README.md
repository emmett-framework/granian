# Granian benchmarks



Run at: Thu 05 Dec 2024, 17:56    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Python version: 3.11    
Granian version: 1.7.0    

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c256) | 983748 | 98563 | 2.595ms | 12.392ms |
| str small (c256) | 973187 | 97468 | 2.624ms | 14.367ms |
| bytes big (c128) | 575437 | 57581 | 2.22ms | 20.235ms |
| str big (c128) | 548202 | 54840 | 2.331ms | 21.674ms |


## Interfaces

Comparison between Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c256) | 992366 | 99422 | 2.572ms | 16.016ms |
| RSGI str (c256) | 993008 | 99490 | 2.569ms | 17.738ms |
| RSGI echo (c512) | 518272 | 52032 | 9.831ms | 22.896ms |
| ASGI bytes (c256) | 752313 | 75350 | 3.389ms | 34.436ms |
| ASGI str (c128) | 757699 | 75816 | 1.687ms | 10.648ms |
| ASGI echo (c512) | 463665 | 46478 | 10.993ms | 41.452ms |
| WSGI bytes (c512) | 638348 | 64077 | 7.98ms | 39.408ms |
| WSGI str (c256) | 617444 | 61883 | 4.132ms | 16.222ms |
| WSGI echo (c512) | 586371 | 58826 | 8.69ms | 47.028ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using 4bytes plain text response.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 [GET] (c256) | 999863 | 100135 | 2.554ms | 18.075ms |
| HTTP/1 [POST] (c512) | 522805 | 52406 | 9.757ms | 41.837ms |
| HTTP/2 [GET] (c256) | 900927 | 90252 | 2.834ms | 52.874ms |
| HTTP/2 [POST] (c512) | 552961 | 55487 | 9.216ms | 79.764ms |


## File responses

Comparison between Granian application protocols using 95bytes image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c256) | 784958 | 78604 | 3.254ms | 18.597ms |
| ASGI (c512) | 333716 | 33488 | 15.273ms | 56.893ms |
| ASGI pathsend (c512) | 665802 | 66783 | 7.656ms | 60.436ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
- [Python versions](./pyver.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
