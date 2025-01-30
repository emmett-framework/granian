# Granian benchmarks



## VS 3rd party comparison

Run at: Thu 30 Jan 2025, 02:50    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Python version: 3.11    
Granian version: 1.7.6

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c128) | 718984 | 71926 | 1.778ms | 10.975ms |
| Granian Asgi [POST] (c64) | 508003 | 50801 | 1.258ms | 2.088ms |
| Uvicorn H11 [GET] (c128) | 102848 | 10290 | 12.417ms | 372.288ms |
| Uvicorn H11 [POST] (c128) | 91224 | 9128 | 13.998ms | 353.725ms |
| Uvicorn Httptools [GET] (c128) | 467172 | 46737 | 2.736ms | 14.645ms |
| Uvicorn Httptools [POST] (c128) | 431909 | 43204 | 2.959ms | 10.001ms |
| Hypercorn [GET] (c128) | 67596 | 6764 | 18.884ms | 128.518ms |
| Hypercorn [POST] (c128) | 61623 | 6164 | 20.725ms | 118.589ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c128) | 638035 | 63843 | 2.003ms | 15.961ms |
| Granian Wsgi [POST] (c256) | 598326 | 59949 | 4.265ms | 15.323ms |
| Gunicorn Gthread [GET] (c64) | 56594 | 5660 | 11.283ms | 26.561ms |
| Gunicorn Gthread [POST] (c64) | 55218 | 5522 | 11.575ms | 25.991ms |
| Gunicorn Gevent [GET] (c256) | 87832 | 8797 | 1.423ms | 9872.811ms |
| Gunicorn Gevent [POST] (c128) | 81247 | 8128 | 1.146ms | 5752.792ms |
| Uwsgi [GET] (c512) | 164696 | 16553 | 16.701ms | 7355.456ms |
| Uwsgi [POST] (c64) | 164497 | 16449 | 3.887ms | 5.072ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c128) | 726241 | 72651 | 1.759ms | 17.512ms |
| Granian Asgi [POST] (c64) | 516846 | 51688 | 1.236ms | 3.389ms |
| Hypercorn [GET] (c64) | 43291 | 4329 | 14.749ms | 43.753ms |
| Hypercorn [POST] (c64) | 38619 | 3862 | 16.539ms | 48.591ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c512) | 659241 | 66128 | 7.729ms | 48.28ms |
| Uvicorn H11 (c128) | 102330 | 10239 | 12.482ms | 276.461ms |
| Uvicorn Httptools (c128) | 264912 | 26501 | 4.826ms | 28.005ms |
| Hypercorn (c128) | 67340 | 6738 | 18.959ms | 132.888ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 379870 | 38098 | 13.412ms | 41.236ms |
| Granian Rsgi 100ms (c512) | 50067 | 5022 | 101.413ms | 113.043ms |
| Granian Asgi 10ms (c512) | 359047 | 36001 | 14.196ms | 62.374ms |
| Granian Asgi 100ms (c512) | 50095 | 5021 | 101.285ms | 108.246ms |
| Granian Wsgi 10ms (c512) | 269888 | 27051 | 18.881ms | 65.285ms |
| Granian Wsgi 100ms (c512) | 50688 | 5081 | 100.287ms | 116.477ms |
| Uvicorn Httptools 10ms (c512) | 340358 | 34107 | 14.982ms | 233.988ms |
| Uvicorn Httptools 100ms (c512) | 50213 | 5032 | 100.969ms | 120.684ms |
| Hypercorn 10ms (c128) | 68128 | 6819 | 18.728ms | 128.653ms |
| Hypercorn 100ms (c128) | 67747 | 6778 | 18.846ms | 111.662ms |
| Gunicorn Gevent 10ms (c128) | 79684 | 7971 | 16.034ms | 27.78ms |
| Gunicorn Gevent 100ms (c512) | 50284 | 5041 | 100.938ms | 145.848ms |
| Uwsgi 10ms (c512) | 164183 | 16491 | 16.501ms | 7366.024ms |
| Uwsgi 100ms (c128) | 165192 | 16526 | 7.672ms | 3334.318ms |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

| Clients | Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- | --- |
| 8 | Granian Rsgi | 1124405 | 214281 | 241066 |
| 8 | Granian Asgi | 1163605 | 210153 | 236422 |
| 8 | Uvicorn H11 | 716717 | 136249 | 153280 |
| 8 | Hypercorn | 670931 | 98839 | 111194 |
| 16 | Granian Rsgi | 2180501 | 229868 | 244235 |
| 16 | Granian Asgi | 2178232 | 219395 | 233107 |
| 16 | Uvicorn H11 | 1466562 | 139319 | 148026 |
| 16 | Hypercorn | 1448514 | 105812 | 112425 |
| 32 | Granian Rsgi | 3946565 | 235608 | 242970 |
| 32 | Granian Asgi | 3875551 | 234267 | 241588 |
| 32 | Uvicorn H11 | N/A | N/A | N/A |
| 32 | Hypercorn | N/A | N/A | N/A |

