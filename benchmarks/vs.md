# Granian benchmarks



## VS 3rd party comparison

Run at: Mon 15 Apr 2024, 18:44    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.2.3    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c64) | 551380 | 36762 | 1.738ms | 3.825ms |
| Granian Asgi [POST] (c128) | 357168 | 23826 | 5.361ms | 19.084ms |
| Uvicorn H11 [GET] (c128) | 115703 | 7719 | 16.535ms | 40.725ms |
| Uvicorn H11 [POST] (c64) | 104439 | 6963 | 9.176ms | 25.187ms |
| Uvicorn Httptools [GET] (c128) | 541214 | 36117 | 3.536ms | 19.855ms |
| Uvicorn Httptools [POST] (c128) | 504125 | 33642 | 3.795ms | 28.211ms |
| Hypercorn [GET] (c128) | 74803 | 4991 | 25.575ms | 37.583ms |
| Hypercorn [POST] (c128) | 68276 | 4554 | 28.019ms | 47.435ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c64) | 582998 | 38871 | 1.644ms | 4.146ms |
| Granian Wsgi [POST] (c64) | 525354 | 35028 | 1.824ms | 4.838ms |
| Gunicorn Gthread [GET] (c32) | 59389 | 3959 | 8.078ms | 9.318ms |
| Gunicorn Gthread [POST] (c32) | 57913 | 3861 | 8.284ms | 9.777ms |
| Gunicorn Gevent [GET] (c64) | 94411 | 6295 | 8.42ms | 6790.479ms |
| Gunicorn Gevent [POST] (c128) | 90840 | 6061 | 13.319ms | 11953.935ms |
| Uwsgi [GET] (c64) | 108358 | 7225 | 8.843ms | 20.987ms |
| Uwsgi [POST] (c32) | 107442 | 7163 | 4.465ms | 12.032ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c128) | 544685 | 36342 | 3.51ms | 48.503ms |
| Granian Asgi [POST] (c64) | 302803 | 20190 | 3.164ms | 7.544ms |
| Hypercorn [GET] (c128) | 22313 | 1489 | 85.056ms | 310.829ms |
| Hypercorn [POST] (c64) | 40852 | 2724 | 23.437ms | 73.216ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c256) | 274168 | 18319 | 13.923ms | 68.447ms |
| Uvicorn H11 (c64) | 116771 | 7785 | 8.209ms | 20.761ms |
| Uvicorn Httptools (c128) | 308987 | 20615 | 6.193ms | 28.118ms |
| Hypercorn (c128) | 73342 | 4893 | 26.084ms | 38.552ms |

