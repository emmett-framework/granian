# Granian benchmarks



## VS 3rd party comparison

Run at: Sun 01 Dec 2024, 23:57    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.7.0    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c256) | 293232 | 29401 | 8.664ms | 70.371ms |
| Granian Asgi [POST] (c128) | 183324 | 18352 | 6.944ms | 41.357ms |
| Uvicorn H11 [GET] (c128) | 75309 | 7542 | 16.901ms | 33.585ms |
| Uvicorn H11 [POST] (c128) | 67231 | 6737 | 18.923ms | 43.07ms |
| Uvicorn Httptools [GET] (c128) | 336426 | 33675 | 3.79ms | 23.261ms |
| Uvicorn Httptools [POST] (c128) | 312870 | 31338 | 4.071ms | 20.711ms |
| Hypercorn [GET] (c128) | 48221 | 4827 | 26.398ms | 35.11ms |
| Hypercorn [POST] (c128) | 44687 | 4473 | 28.488ms | 32.279ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c64) | 377696 | 37779 | 1.689ms | 4.502ms |
| Granian Wsgi [POST] (c64) | 352448 | 35251 | 1.811ms | 4.622ms |
| Gunicorn Gthread [GET] (c64) | 37150 | 3716 | 17.165ms | 22.192ms |
| Gunicorn Gthread [POST] (c64) | 34736 | 3474 | 18.377ms | 22.772ms |
| Gunicorn Gevent [GET] (c512) | 61542 | 6191 | 7.503ms | 9900.82ms |
| Gunicorn Gevent [POST] (c64) | 58378 | 5839 | 8.357ms | 5561.704ms |
| Uwsgi [GET] (c256) | 71972 | 7213 | 34.585ms | 3347.144ms |
| Uwsgi [POST] (c256) | 70943 | 7115 | 33.959ms | 6655.117ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c256) | 278676 | 27943 | 9.121ms | 98.216ms |
| Granian Asgi [POST] (c256) | 167348 | 16788 | 15.172ms | 97.129ms |
| Hypercorn [GET] (c64) | 31975 | 3198 | 19.939ms | 44.669ms |
| Hypercorn [POST] (c64) | 27788 | 2780 | 22.93ms | 52.417ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c512) | 229502 | 23074 | 22.034ms | 126.181ms |
| Uvicorn H11 (c64) | 74008 | 7402 | 8.619ms | 20.852ms |
| Uvicorn Httptools (c64) | 185926 | 18597 | 3.432ms | 6.473ms |
| Hypercorn (c128) | 46120 | 4617 | 27.599ms | 42.77ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 289872 | 29137 | 17.488ms | 112.354ms |
| Granian Rsgi 100ms (c512) | 49468 | 4976 | 101.832ms | 165.88ms |
| Granian Asgi 10ms (c512) | 242743 | 24404 | 20.876ms | 114.145ms |
| Granian Asgi 100ms (c512) | 49896 | 5012 | 101.187ms | 144.957ms |
| Granian Wsgi 10ms (c128) | 111026 | 11120 | 11.47ms | 38.543ms |
| Granian Wsgi 100ms (c512) | 50321 | 5058 | 100.288ms | 135.461ms |
| Uvicorn Httptools 10ms (c512) | 232101 | 23325 | 21.83ms | 98.894ms |
| Uvicorn Httptools 100ms (c512) | 49986 | 5022 | 100.887ms | 191.428ms |
| Hypercorn 10ms (c128) | 47868 | 4792 | 26.588ms | 43.619ms |
| Hypercorn 100ms (c128) | 49555 | 4962 | 25.691ms | 41.233ms |
| Gunicorn Gevent 10ms (c64) | 57246 | 5725 | 11.151ms | 21.612ms |
| Gunicorn Gevent 100ms (c512) | 49098 | 4938 | 102.694ms | 181.789ms |
| Uwsgi 10ms (c64) | 72340 | 7235 | 8.821ms | 23.697ms |
| Uwsgi 100ms (c64) | 72123 | 7213 | 8.85ms | 19.812ms |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

| Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- |
| Granian Rsgi (c16) | 1128866 | 76743 | 81539 |
| Granian Asgi (c16) | 901094 | 72809 | 77360 |
| Uvicorn H11 (c8) | 560592 | 105928 | 119169 |

