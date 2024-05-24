# Granian benchmarks



## VS 3rd party comparison

Run at: Fri 24 May 2024, 14:32    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.3.2    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c32) | 632544 | 42169 | 0.758ms | 10.208ms |
| Granian Asgi [POST] (c128) | 386827 | 25809 | 4.946ms | 32.645ms |
| Uvicorn H11 [GET] (c128) | 118452 | 7904 | 16.156ms | 38.248ms |
| Uvicorn H11 [POST] (c128) | 106516 | 7106 | 17.978ms | 40.068ms |
| Uvicorn Httptools [GET] (c128) | 541890 | 36175 | 3.53ms | 20.76ms |
| Uvicorn Httptools [POST] (c128) | 500953 | 33424 | 3.822ms | 20.828ms |
| Hypercorn [GET] (c128) | 74704 | 4985 | 25.601ms | 41.465ms |
| Hypercorn [POST] (c128) | 67297 | 4491 | 28.412ms | 53.161ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c256) | 609880 | 40737 | 6.264ms | 64.326ms |
| Granian Wsgi [POST] (c128) | 530524 | 35395 | 3.609ms | 20.136ms |
| Gunicorn Gthread [GET] (c32) | 59742 | 3983 | 8.03ms | 9.413ms |
| Gunicorn Gthread [POST] (c32) | 57090 | 3806 | 8.403ms | 11.267ms |
| Gunicorn Gevent [GET] (c32) | 94521 | 6301 | 4.554ms | 2839.894ms |
| Gunicorn Gevent [POST] (c256) | 87844 | 5867 | 25.51ms | 13754.327ms |
| Uwsgi [GET] (c256) | 107451 | 7173 | 34.687ms | 6550.371ms |
| Uwsgi [POST] (c64) | 107136 | 7143 | 8.946ms | 21.903ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c64) | 550512 | 36706 | 1.74ms | 4.892ms |
| Granian Asgi [POST] (c64) | 314702 | 20983 | 3.045ms | 7.485ms |
| Hypercorn [GET] (c64) | 22437 | 1496 | 42.593ms | 158.838ms |
| Hypercorn [POST] (c64) | 42597 | 2840 | 22.476ms | 45.309ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c256) | 428206 | 28593 | 8.929ms | 60.664ms |
| Uvicorn H11 (c128) | 118950 | 7935 | 16.088ms | 27.739ms |
| Uvicorn Httptools (c128) | 309412 | 20644 | 6.188ms | 24.089ms |
| Hypercorn (c128) | 74027 | 4938 | 25.847ms | 41.455ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c256) | 345617 | 23100 | 11.042ms | 75.88ms |
| Granian Rsgi 100ms (c256) | 37888 | 2530 | 100.629ms | 144.07ms |
| Granian Asgi 10ms (c256) | 342472 | 22870 | 11.15ms | 71.227ms |
| Granian Asgi 100ms (c256) | 37891 | 2530 | 100.598ms | 147.491ms |
| Granian Wsgi 10ms (c32) | 1478 | 99 | 321.226ms | 325.649ms |
| Granian Wsgi 100ms (c128) | 116 | 8 | 9120.449ms | 14880.201ms |
| Uvicorn Httptools 10ms (c256) | 338141 | 22590 | 11.301ms | 59.64ms |
| Uvicorn Httptools 100ms (c256) | 37894 | 2531 | 100.658ms | 128.533ms |
| Hypercorn 10ms (c128) | 73701 | 4917 | 25.955ms | 38.399ms |
| Hypercorn 100ms (c128) | 73125 | 4880 | 26.157ms | 41.109ms |
| Gunicorn Gevent 10ms (c64) | 87534 | 5836 | 10.949ms | 25.004ms |
| Gunicorn Gevent 100ms (c256) | 37674 | 2516 | 100.994ms | 179.057ms |
| Uwsgi 10ms (c256) | 107450 | 7174 | 34.813ms | 6710.914ms |
| Uwsgi 100ms (c256) | 107184 | 7156 | 34.704ms | 3299.761ms |

