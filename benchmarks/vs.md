# Granian benchmarks



## VS 3rd party comparison

Run at: Wed 01 May 2024, 21:13    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.3.1    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c32) | 649320 | 43287 | 0.739ms | 3.766ms |
| Granian Asgi [POST] (c256) | 385566 | 25755 | 9.903ms | 73.83ms |
| Uvicorn H11 [GET] (c128) | 118796 | 7928 | 16.1ms | 42.978ms |
| Uvicorn H11 [POST] (c64) | 105397 | 7027 | 9.091ms | 38.113ms |
| Uvicorn Httptools [GET] (c128) | 549961 | 36690 | 3.481ms | 27.95ms |
| Uvicorn Httptools [POST] (c128) | 512661 | 34208 | 3.734ms | 20.682ms |
| Hypercorn [GET] (c128) | 75494 | 5038 | 25.33ms | 40.568ms |
| Hypercorn [POST] (c128) | 67814 | 4524 | 28.19ms | 61.026ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c64) | 609496 | 40638 | 1.572ms | 4.383ms |
| Granian Wsgi [POST] (c64) | 561435 | 37433 | 1.707ms | 4.389ms |
| Gunicorn Gthread [GET] (c32) | 59409 | 3960 | 8.076ms | 9.512ms |
| Gunicorn Gthread [POST] (c32) | 56812 | 3787 | 8.445ms | 12.686ms |
| Gunicorn Gevent [GET] (c256) | 95446 | 6375 | 10.678ms | 14883.455ms |
| Gunicorn Gevent [POST] (c256) | 89198 | 5955 | 13.945ms | 14817.326ms |
| Uwsgi [GET] (c128) | 109286 | 7292 | 17.393ms | 2081.493ms |
| Uwsgi [POST] (c256) | 108703 | 7259 | 34.551ms | 4305.52ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c128) | 541473 | 36123 | 3.537ms | 22.256ms |
| Granian Asgi [POST] (c64) | 314174 | 20949 | 3.049ms | 6.554ms |
| Hypercorn [GET] (c32) | 22387 | 1492 | 21.409ms | 78.66ms |
| Hypercorn [POST] (c64) | 42373 | 2825 | 22.6ms | 48.051ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c128) | 441700 | 29469 | 4.335ms | 30.157ms |
| Uvicorn H11 (c128) | 118445 | 7904 | 16.151ms | 27.365ms |
| Uvicorn Httptools (c128) | 313122 | 20897 | 6.11ms | 23.935ms |
| Hypercorn (c128) | 75973 | 5069 | 25.184ms | 36.181ms |

