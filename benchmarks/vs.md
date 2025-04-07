# Granian benchmarks



## VS 3rd party comparison

Run at: Mon 07 Apr 2025, 11:38    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Python version: 3.12    
Granian version: 2.2.2

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c128) | 609514 | 61005 | 2.095ms | 27.85ms |
| Granian Asgi echo 10KB (iter) (c64) | 274220 | 27425 | 2.328ms | 4.363ms |
| Uvicorn H11 get 10KB (c64) | 95438 | 9543 | 6.698ms | 18.401ms |
| Uvicorn H11 echo 10KB (iter) (c64) | 82258 | 8226 | 7.766ms | 19.364ms |
| Uvicorn Httptools get 10KB (c128) | 394122 | 39459 | 3.237ms | 16.513ms |
| Uvicorn Httptools echo 10KB (iter) (c128) | 344028 | 34435 | 3.708ms | 16.547ms |
| Hypercorn get 10KB (c128) | 65468 | 6552 | 19.464ms | 21.873ms |
| Hypercorn echo 10KB (iter) (c128) | 57517 | 5755 | 22.177ms | 36.325ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi get 1KB (c64) | 636830 | 63688 | 1.003ms | 1.667ms |
| Granian Wsgi echo 1KB (c64) | 586027 | 58610 | 1.089ms | 1.843ms |
| Gunicorn Gthread get 1KB (c64) | 62499 | 6251 | 10.208ms | 27.621ms |
| Gunicorn Gthread echo 1KB (c64) | 57357 | 5736 | 11.132ms | 27.106ms |
| Gunicorn Gevent get 1KB (c64) | 94284 | 9429 | 3.429ms | 6260.77ms |
| Gunicorn Gevent echo 1KB (c64) | 86794 | 8680 | 4.747ms | 6163.71ms |
| Uwsgi get 1KB (c64) | 168173 | 16820 | 3.794ms | 7.358ms |
| Uwsgi echo 1KB (c128) | 162186 | 16231 | 7.804ms | 2085.755ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c256) | 598504 | 60017 | 4.26ms | 66.409ms |
| Granian Asgi echo 10KB (iter) (c512) | 120238 | 12083 | 42.177ms | 116.207ms |
| Hypercorn get 10KB (c128) | 40883 | 4095 | 31.164ms | 355.431ms |
| Hypercorn echo 10KB (iter) (c128) | 31726 | 3174 | 40.164ms | 71.249ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c512) | 284344 | 28540 | 17.831ms | 235.013ms |
| Uvicorn H11 (c128) | 93916 | 9403 | 13.58ms | 299.406ms |
| Uvicorn Httptools (c64) | 154439 | 15444 | 4.136ms | 9.93ms |
| Hypercorn (c128) | 64703 | 6476 | 19.721ms | 189.995ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 359814 | 36097 | 14.151ms | 65.856ms |
| Granian Rsgi 100ms (c512) | 49825 | 4999 | 101.882ms | 118.535ms |
| Granian Asgi 10ms (c512) | 335905 | 33777 | 15.123ms | 63.396ms |
| Granian Asgi 100ms (c512) | 49840 | 5006 | 101.43ms | 105.255ms |
| Granian Wsgi 10ms (c256) | 206612 | 20714 | 12.32ms | 41.963ms |
| Granian Wsgi 100ms (c512) | 50688 | 5083 | 100.192ms | 107.739ms |
| Uvicorn Httptools 10ms (c512) | 307387 | 30850 | 16.557ms | 64.484ms |
| Uvicorn Httptools 100ms (c512) | 50371 | 5052 | 100.719ms | 112.446ms |
| Hypercorn 10ms (c128) | 64640 | 6472 | 19.711ms | 22.199ms |
| Hypercorn 100ms (c128) | 65091 | 6515 | 19.602ms | 193.631ms |
| Gunicorn Gevent 10ms (c128) | 85109 | 8518 | 14.997ms | 26.545ms |
| Gunicorn Gevent 100ms (c512) | 50277 | 5047 | 100.941ms | 138.139ms |
| Uwsgi 10ms (c128) | 164358 | 16459 | 7.676ms | 1670.195ms |
| Uwsgi 100ms (c512) | 162038 | 16299 | 20.571ms | 6870.561ms |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

| Clients | Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- | --- |
| 8 | Granian Rsgi | 1029991 | 202749 | 228093 |
| 8 | Granian Asgi | 1023277 | 195250 | 219657 |
| 8 | Uvicorn H11 | 627414 | 124792 | 140391 |
| 8 | Hypercorn | 568923 | 90745 | 102088 |
| 16 | Granian Rsgi | 1932155 | 222312 | 236207 |
| 16 | Granian Asgi | 1951899 | 217395 | 230982 |
| 16 | Uvicorn H11 | 1236645 | 129593 | 137693 |
| 16 | Hypercorn | 1202992 | 96312 | 102332 |
| 32 | Granian Rsgi | 3522612 | 223784 | 230777 |
| 32 | Granian Asgi | 3458683 | 220858 | 227760 |
| 32 | Uvicorn H11 | N/A | N/A | N/A |
| 32 | Hypercorn | N/A | N/A | N/A |

