
# Granian benchmarks

Run at: Thu 11 Apr 2024, 23:57
Environment: GHA (CPUs: 4)
Python version: 3.11
Granian version: 1.2.2

## RSGI response types

> RSGI plain text response comparison using protocol `response_str` and `response_bytes`.
> The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c128) | 648848 | 43288.911482463416 | 2.951ms | 19.395ms |
| str small (c32) | 643329 | 42888.48164208683 | 0.746ms | 2.624ms |
| bytes big (c64) | 439301 | 29287.413447600906 | 2.182ms | 5.88ms |
| str big (c64) | 435202 | 29015.519534017378 | 2.202ms | 5.95ms |


## Interfaces

> Comparison between Granian application protocols using 4bytes plain text response.
> Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.
> ASGI and WSGI responses are always returned as bytes by the application.
> The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c256) | 643593 | 43007.9565068363 | 5.93ms | 73.348ms |
| RSGI str (c256) | 707989 | 47284.36722671098 | 5.399ms | 62.555ms |
| RSGI echo (c32) | 5421 | 361.38949416647824 | 87.126ms | 1187.761ms |
| ASGI bytes (c32) | 648177 | 43210.61895583987 | 0.74ms | 2.936ms |
| ASGI str (c64) | 637827 | 42526.31089054001 | 1.502ms | 3.716ms |
| ASGI echo (c32) | 4092 | 272.78868492539436 | 114.863ms | 638.969ms |
| WSGI bytes (c64) | 598374 | 39896.94228036523 | 1.601ms | 4.083ms |
| WSGI str (c32) | 652752 | 43516.734046037884 | 0.735ms | 4.237ms |
| WSGI echo (c256) | 550268 | 36760.53202538349 | 6.935ms | 81.874ms |


## Interfaces

> Comparison between Granian application protocols using 4bytes plain text response.
> Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.
> ASGI and WSGI responses are always returned as bytes by the application.
> The "echo" request is a 4bytes POST request responding with the same body.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c128) | 535806 | 35746.936228119324 | 3.573ms | 23.796ms |
| ASGI (c128) | 227362 | 15168.320011201764 | 8.416ms | 27.736ms |
| ASGI pathsend (c256) | 0 | 0 | N/A | N/A |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
