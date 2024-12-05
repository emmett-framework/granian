# Granian benchmarks



## VS 3rd party comparison

Run at: Thu 05 Dec 2024, 18:18    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Python version: 3.11    
Granian version: 1.7.0    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c256) | 772629 | 77369 | 3.306ms | 14.867ms |
| Granian Asgi [POST] (c512) | 467239 | 46892 | 10.904ms | 54.41ms |
| Uvicorn H11 [GET] (c128) | 103070 | 10311 | 12.4ms | 287.805ms |
| Uvicorn H11 [POST] (c128) | 91982 | 9203 | 13.883ms | 57.676ms |
| Uvicorn Httptools [GET] (c128) | 470471 | 47072 | 2.716ms | 9.588ms |
| Uvicorn Httptools [POST] (c128) | 434383 | 43463 | 2.942ms | 9.604ms |
| Hypercorn [GET] (c128) | 66907 | 6697 | 19.069ms | 133.518ms |
| Hypercorn [POST] (c128) | 61390 | 6142 | 20.785ms | 35.274ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c256) | 636116 | 63712 | 4.014ms | 12.569ms |
| Granian Wsgi [POST] (c64) | 589361 | 58936 | 1.085ms | 1.896ms |
| Gunicorn Gthread [GET] (c64) | 60546 | 6055 | 10.557ms | 31.365ms |
| Gunicorn Gthread [POST] (c64) | 58213 | 5821 | 10.979ms | 24.971ms |
| Gunicorn Gevent [GET] (c256) | 89775 | 8990 | 0.982ms | 7874.828ms |
| Gunicorn Gevent [POST] (c256) | 82090 | 8219 | 1.795ms | 9666.629ms |
| Uwsgi [GET] (c128) | 164687 | 16477 | 7.697ms | 6618.283ms |
| Uwsgi [POST] (c512) | 162280 | 16297 | 17.014ms | 7590.528ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c512) | 745356 | 74798 | 6.822ms | 103.548ms |
| Granian Asgi [POST] (c256) | 435588 | 43634 | 5.86ms | 53.054ms |
| Hypercorn [GET] (c64) | 43591 | 4359 | 14.659ms | 58.287ms |
| Hypercorn [POST] (c128) | 38384 | 3841 | 33.229ms | 233.31ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c256) | 661307 | 66233 | 3.86ms | 17.939ms |
| Uvicorn H11 (c128) | 102743 | 10281 | 12.428ms | 21.549ms |
| Uvicorn Httptools (c128) | 266556 | 26665 | 4.795ms | 13.905ms |
| Hypercorn (c128) | 68920 | 6895 | 18.517ms | 29.891ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 410365 | 41175 | 12.415ms | 41.443ms |
| Granian Rsgi 100ms (c512) | 50176 | 5031 | 101.311ms | 114.784ms |
| Granian Asgi 10ms (c512) | 403514 | 40462 | 12.637ms | 40.681ms |
| Granian Asgi 100ms (c512) | 50231 | 5034 | 101.23ms | 111.854ms |
| Granian Wsgi 10ms (c512) | 269512 | 27021 | 18.898ms | 59.231ms |
| Granian Wsgi 100ms (c512) | 50688 | 5079 | 100.319ms | 128.43ms |
| Uvicorn Httptools 10ms (c512) | 340936 | 34190 | 14.943ms | 214.973ms |
| Uvicorn Httptools 100ms (c512) | 50307 | 5042 | 100.884ms | 118.489ms |
| Hypercorn 10ms (c128) | 66669 | 6671 | 19.153ms | 136.363ms |
| Hypercorn 100ms (c128) | 67617 | 6764 | 18.895ms | 134.401ms |
| Gunicorn Gevent 10ms (c128) | 80658 | 8071 | 15.833ms | 33.223ms |
| Gunicorn Gevent 100ms (c512) | 50295 | 5045 | 100.787ms | 140.546ms |
| Uwsgi 10ms (c64) | 164856 | 16486 | 3.878ms | 5.949ms |
| Uwsgi 100ms (c256) | 164544 | 16487 | 11.057ms | 6709.18ms |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

| Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- |
| Granian Rsgi (c16) | 2279821 | 242097 | 257228 |
| Granian Asgi (c32) | 3876720 | 240056 | 247558 |
| Uvicorn H11 (c8) | 757973 | 134512 | 151326 |
| Hypercorn (c16) | 1456033 | 106542 | 113200 |

