# Granian benchmarks



## VS 3rd party comparison

Run at: Wed 04 Dec 2024, 18:15    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.7.0    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c256) | 282988 | 28375 | 8.978ms | 78.289ms |
| Granian Asgi [POST] (c128) | 193588 | 19376 | 6.584ms | 24.138ms |
| Uvicorn H11 [GET] (c64) | 75369 | 7538 | 8.473ms | 21.3ms |
| Uvicorn H11 [POST] (c128) | 67516 | 6760 | 18.859ms | 26.593ms |
| Uvicorn Httptools [GET] (c128) | 339240 | 33971 | 3.754ms | 23.82ms |
| Uvicorn Httptools [POST] (c128) | 314280 | 31469 | 4.053ms | 24.078ms |
| Hypercorn [GET] (c128) | 48636 | 4868 | 26.174ms | 32.408ms |
| Hypercorn [POST] (c128) | 44571 | 4463 | 28.557ms | 51.212ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c64) | 376877 | 37694 | 1.694ms | 4.015ms |
| Granian Wsgi [POST] (c128) | 347029 | 34756 | 3.668ms | 33.945ms |
| Gunicorn Gthread [GET] (c64) | 37167 | 3717 | 17.172ms | 23.152ms |
| Gunicorn Gthread [POST] (c64) | 35142 | 3515 | 18.158ms | 20.987ms |
| Gunicorn Gevent [GET] (c256) | 62101 | 6228 | 19.005ms | 6451.327ms |
| Gunicorn Gevent [POST] (c128) | 59166 | 5924 | 9.817ms | 9695.539ms |
| Uwsgi [GET] (c128) | 71157 | 7129 | 17.748ms | 2078.48ms |
| Uwsgi [POST] (c64) | 70899 | 7091 | 9.002ms | 20.959ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c256) | 269903 | 27091 | 9.393ms | 104.297ms |
| Granian Asgi [POST] (c128) | 167559 | 16779 | 7.597ms | 42.624ms |
| Hypercorn [GET] (c64) | 31299 | 3131 | 20.377ms | 47.789ms |
| Hypercorn [POST] (c64) | 26538 | 2654 | 24.035ms | 62.894ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c256) | 243559 | 24408 | 10.445ms | 52.161ms |
| Uvicorn H11 (c128) | 75632 | 7573 | 16.832ms | 24.585ms |
| Uvicorn Httptools (c128) | 193682 | 19396 | 6.576ms | 18.237ms |
| Hypercorn (c128) | 49768 | 4983 | 25.584ms | 41.56ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 271511 | 27310 | 18.648ms | 134.376ms |
| Granian Rsgi 100ms (c512) | 49454 | 4970 | 101.984ms | 156.255ms |
| Granian Asgi 10ms (c512) | 241470 | 24295 | 20.958ms | 131.146ms |
| Granian Asgi 100ms (c512) | 49805 | 5005 | 101.293ms | 147.57ms |
| Granian Wsgi 10ms (c128) | 108495 | 10865 | 11.719ms | 38.516ms |
| Granian Wsgi 100ms (c512) | 50324 | 5068 | 100.289ms | 135.645ms |
| Uvicorn Httptools 10ms (c256) | 216367 | 21695 | 11.744ms | 56.174ms |
| Uvicorn Httptools 100ms (c512) | 49998 | 5028 | 100.784ms | 183.893ms |
| Hypercorn 10ms (c128) | 49620 | 4968 | 25.645ms | 42.826ms |
| Hypercorn 100ms (c128) | 48313 | 4838 | 26.335ms | 49.255ms |
| Gunicorn Gevent 10ms (c128) | 56294 | 5636 | 22.625ms | 54.91ms |
| Gunicorn Gevent 100ms (c512) | 46933 | 4720 | 107.374ms | 176.384ms |
| Uwsgi 10ms (c128) | 71376 | 7145 | 17.712ms | 2068.695ms |
| Uwsgi 100ms (c128) | 71318 | 7141 | 17.753ms | 3097.96ms |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

| Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- |
| Granian Rsgi (c16) | 948626 | 88127 | 93635 |
| Granian Asgi (c16) | 927519 | 83739 | 88972 |
| Uvicorn H11 (c8) | 566956 | 104606 | 117682 |

