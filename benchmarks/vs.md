# Granian benchmarks



## VS 3rd party comparison

Run at: Sat 25 May 2024, 14:51    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.3.2    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c256) | 630480 | 42107 | 6.061ms | 61.358ms |
| Granian Asgi [POST] (c128) | 363402 | 24252 | 5.263ms | 27.586ms |
| Uvicorn H11 [GET] (c128) | 117610 | 7846 | 16.27ms | 26.345ms |
| Uvicorn H11 [POST] (c128) | 106968 | 7138 | 17.878ms | 46.978ms |
| Uvicorn Httptools [GET] (c128) | 546194 | 36438 | 3.506ms | 20.702ms |
| Uvicorn Httptools [POST] (c128) | 504698 | 33672 | 3.795ms | 26.673ms |
| Hypercorn [GET] (c128) | 73494 | 4903 | 26.034ms | 31.951ms |
| Hypercorn [POST] (c128) | 67027 | 4472 | 28.54ms | 46.604ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c64) | 259201 | 17282 | 3.697ms | 42.04ms |
| Granian Wsgi [POST] (c64) | 202583 | 13507 | 4.73ms | 52.536ms |
| Gunicorn Gthread [GET] (c64) | 55981 | 3732 | 17.11ms | 19.485ms |
| Gunicorn Gthread [POST] (c64) | 53806 | 3588 | 17.801ms | 20.064ms |
| Gunicorn Gevent [GET] (c64) | 95495 | 6367 | 8.334ms | 7681.561ms |
| Gunicorn Gevent [POST] (c512) | 88399 | 5915 | 16.764ms | 14854.23ms |
| Uwsgi [GET] (c128) | 107704 | 7184 | 17.69ms | 3067.247ms |
| Uwsgi [POST] (c128) | 107408 | 7167 | 17.698ms | 2066.874ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c64) | 541499 | 36104 | 1.769ms | 7.323ms |
| Granian Asgi [POST] (c64) | 317689 | 21183 | 3.017ms | 6.869ms |
| Hypercorn [GET] (c128) | 22800 | 1522 | 83.355ms | 302.998ms |
| Hypercorn [POST] (c64) | 41995 | 2800 | 22.802ms | 53.132ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c512) | 388510 | 26005 | 19.622ms | 138.405ms |
| Uvicorn H11 (c128) | 121020 | 8074 | 15.806ms | 30.144ms |
| Uvicorn Httptools (c64) | 304268 | 20286 | 3.15ms | 6.181ms |
| Hypercorn (c128) | 74988 | 5002 | 25.516ms | 42.627ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 583017 | 38994 | 13.092ms | 113.657ms |
| Granian Rsgi 100ms (c512) | 75539 | 5056 | 100.631ms | 162.819ms |
| Granian Asgi 10ms (c512) | 583922 | 39051 | 13.068ms | 131.136ms |
| Granian Asgi 100ms (c512) | 75527 | 5053 | 100.685ms | 148.895ms |
| Granian Wsgi 10ms (c128) | 178133 | 11885 | 10.737ms | 34.443ms |
| Granian Wsgi 100ms (c512) | 75782 | 5074 | 100.305ms | 173.546ms |
| Uvicorn Httptools 10ms (c512) | 357866 | 23951 | 21.295ms | 104.298ms |
| Uvicorn Httptools 100ms (c512) | 75501 | 5050 | 100.803ms | 198.395ms |
| Hypercorn 10ms (c128) | 73672 | 4916 | 25.959ms | 47.932ms |
| Hypercorn 100ms (c128) | 74252 | 4953 | 25.771ms | 43.482ms |
| Gunicorn Gevent 10ms (c64) | 87939 | 5863 | 10.897ms | 21.133ms |
| Gunicorn Gevent 100ms (c512) | 72919 | 4876 | 104.429ms | 153.739ms |
| Uwsgi 10ms (c256) | 109236 | 7293 | 34.039ms | 13358.946ms |
| Uwsgi 100ms (c512) | 107535 | 7192 | 67.082ms | 8084.354ms |

