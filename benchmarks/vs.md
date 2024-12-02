# Granian benchmarks



## VS 3rd party comparison

Run at: Mon 02 Dec 2024, 00:49    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.7.0    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c256) | 298847 | 29982 | 8.493ms | 76.307ms |
| Granian Asgi [POST] (c512) | 180915 | 18192 | 28.008ms | 119.179ms |
| Uvicorn H11 [GET] (c64) | 75467 | 7548 | 8.456ms | 24.193ms |
| Uvicorn H11 [POST] (c128) | 68575 | 6867 | 18.568ms | 38.929ms |
| Uvicorn Httptools [GET] (c128) | 344987 | 34544 | 3.694ms | 21.338ms |
| Uvicorn Httptools [POST] (c128) | 315700 | 31603 | 4.039ms | 24.235ms |
| Hypercorn [GET] (c128) | 49625 | 4968 | 25.648ms | 30.835ms |
| Hypercorn [POST] (c128) | 45345 | 4539 | 28.067ms | 34.67ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c64) | 378784 | 37885 | 1.685ms | 4.807ms |
| Granian Wsgi [POST] (c64) | 354183 | 35422 | 1.802ms | 4.646ms |
| Gunicorn Gthread [GET] (c64) | 36110 | 3612 | 17.665ms | 19.779ms |
| Gunicorn Gthread [POST] (c64) | 34672 | 3468 | 18.397ms | 23.659ms |
| Gunicorn Gevent [GET] (c64) | 60859 | 6087 | 8.623ms | 3900.876ms |
| Gunicorn Gevent [POST] (c512) | 57087 | 5743 | 15.129ms | 9838.398ms |
| Uwsgi [GET] (c512) | 71037 | 7137 | 61.281ms | 6773.376ms |
| Uwsgi [POST] (c512) | 71620 | 7208 | 64.454ms | 4117.911ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c256) | 279657 | 28050 | 9.077ms | 104.015ms |
| Granian Asgi [POST] (c256) | 168299 | 16865 | 15.113ms | 81.061ms |
| Hypercorn [GET] (c64) | 30958 | 3096 | 20.601ms | 43.075ms |
| Hypercorn [POST] (c64) | 28402 | 2841 | 22.459ms | 48.321ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c256) | 238778 | 23928 | 10.646ms | 64.821ms |
| Uvicorn H11 (c128) | 74581 | 7467 | 17.07ms | 40.65ms |
| Uvicorn Httptools (c128) | 188514 | 18882 | 6.756ms | 31.749ms |
| Hypercorn (c128) | 47046 | 4709 | 27.051ms | 36.45ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 282667 | 28431 | 17.907ms | 127.667ms |
| Granian Rsgi 100ms (c512) | 49664 | 4989 | 101.837ms | 148.575ms |
| Granian Asgi 10ms (c512) | 243263 | 24427 | 20.832ms | 110.69ms |
| Granian Asgi 100ms (c512) | 49732 | 5008 | 101.5ms | 171.688ms |
| Granian Wsgi 10ms (c128) | 111515 | 11165 | 11.417ms | 31.036ms |
| Granian Wsgi 100ms (c512) | 50234 | 5049 | 100.319ms | 161.93ms |
| Uvicorn Httptools 10ms (c512) | 234987 | 23635 | 21.516ms | 124.601ms |
| Uvicorn Httptools 100ms (c512) | 49971 | 5023 | 100.843ms | 204.706ms |
| Hypercorn 10ms (c128) | 49198 | 4926 | 25.88ms | 42.365ms |
| Hypercorn 100ms (c128) | 49752 | 4981 | 25.587ms | 42.032ms |
| Gunicorn Gevent 10ms (c64) | 56941 | 5695 | 11.211ms | 22.057ms |
| Gunicorn Gevent 100ms (c512) | 46908 | 4719 | 107.531ms | 167.511ms |
| Uwsgi 10ms (c256) | 72448 | 7261 | 33.992ms | 3322.826ms |
| Uwsgi 100ms (c64) | 71777 | 7177 | 8.895ms | 24.13ms |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

| Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- |
| Granian Rsgi (c16) | 892181 | 77043 | 81858 |
| Granian Asgi (c16) | 1008343 | 73929 | 78549 |
| Uvicorn H11 (c8) | 551988 | 105831 | 119060 |

