# Granian benchmarks



## VS 3rd party comparison

Run at: Sat 27 Apr 2024, 01:12    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.3.0    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c32) | 610732 | 40715 | 0.785ms | 3.298ms |
| Granian Asgi [POST] (c64) | 380974 | 25402 | 2.515ms | 10.59ms |
| Uvicorn H11 [GET] (c64) | 116198 | 7747 | 8.244ms | 22.306ms |
| Uvicorn H11 [POST] (c32) | 101626 | 6775 | 4.722ms | 13.353ms |
| Uvicorn Httptools [GET] (c128) | 542176 | 36173 | 3.53ms | 22.402ms |
| Uvicorn Httptools [POST] (c128) | 508480 | 33937 | 3.762ms | 29.082ms |
| Hypercorn [GET] (c128) | 73588 | 4909 | 26.002ms | 42.228ms |
| Hypercorn [POST] (c128) | 66906 | 4464 | 28.585ms | 42.11ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c32) | 564229 | 37615 | 0.85ms | 2.388ms |
| Granian Wsgi [POST] (c256) | 491367 | 32813 | 7.777ms | 64.949ms |
| Gunicorn Gthread [GET] (c64) | 56173 | 3745 | 17.055ms | 22.861ms |
| Gunicorn Gthread [POST] (c32) | 54456 | 3630 | 8.809ms | 10.373ms |
| Gunicorn Gevent [GET] (c256) | 94348 | 6303 | 34.176ms | 8768.285ms |
| Gunicorn Gevent [POST] (c128) | 88666 | 5917 | 17.473ms | 6959.865ms |
| Uwsgi [GET] (c128) | 109257 | 7291 | 17.394ms | 3090.637ms |
| Uwsgi [POST] (c128) | 108865 | 7263 | 17.525ms | 2051.174ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c256) | 550392 | 36793 | 6.927ms | 107.67ms |
| Granian Asgi [POST] (c64) | 316795 | 21123 | 3.025ms | 7.35ms |
| Hypercorn [GET] (c256) | 22627 | 1511 | 166.096ms | 637.885ms |
| Hypercorn [POST] (c64) | 42250 | 2817 | 22.674ms | 45.372ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c256) | 284947 | 19040 | 13.406ms | 69.858ms |
| Uvicorn H11 (c64) | 119181 | 7946 | 8.042ms | 20.592ms |
| Uvicorn Httptools (c128) | 311826 | 20807 | 6.138ms | 20.652ms |
| Hypercorn (c128) | 74584 | 4976 | 25.647ms | 71.309ms |

