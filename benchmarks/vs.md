# Granian benchmarks



## VS 3rd party comparison

Run at: Thu 10 Apr 2025, 17:25    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Python version: 3.12    
Granian version: 2.2.4

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c128) | 609397 | 60968 | 2.098ms | 16.515ms |
| Granian Asgi echo 10KB (iter) (c128) | 335117 | 33559 | 3.801ms | 24.72ms |
| Uvicorn H11 get 10KB (c64) | 96825 | 9682 | 6.602ms | 16.267ms |
| Uvicorn H11 echo 10KB (iter) (c64) | 82267 | 8227 | 7.766ms | 19.165ms |
| Uvicorn Httptools get 10KB (c128) | 397163 | 39768 | 3.214ms | 15.434ms |
| Uvicorn Httptools echo 10KB (iter) (c64) | 345364 | 34540 | 1.848ms | 3.746ms |
| Hypercorn get 10KB (c128) | 64487 | 6453 | 19.795ms | 157.371ms |
| Hypercorn echo 10KB (iter) (c128) | 57951 | 5804 | 21.992ms | 229.892ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi get 10KB (c64) | 630338 | 63040 | 1.013ms | 2.242ms |
| Granian Wsgi echo 10KB (iter) (c128) | 470982 | 47125 | 2.712ms | 27.978ms |
| Gunicorn Gthread get 10KB (c64) | 61475 | 6148 | 10.382ms | 26.972ms |
| Gunicorn Gthread echo 10KB (iter) (c64) | 46615 | 4662 | 13.701ms | 26.975ms |
| Gunicorn Gevent get 10KB (c64) | 93307 | 9331 | 5.748ms | 6256.879ms |
| Gunicorn Gevent echo 10KB (iter) (c64) | 62167 | 6217 | 4.655ms | 7646.817ms |
| Uwsgi get 10KB (c128) | 169540 | 16980 | 7.467ms | 2094.377ms |
| Uwsgi echo 10KB (iter) (c64) | 191900 | 19193 | 3.325ms | 5.855ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c128) | 597891 | 59847 | 2.134ms | 34.144ms |
| Granian Asgi echo 10KB (iter) (c512) | 121287 | 12197 | 41.762ms | 151.463ms |
| Hypercorn get 10KB (c128) | 41220 | 4127 | 30.917ms | 450.366ms |
| Hypercorn echo 10KB (iter) (c128) | 31542 | 3157 | 40.395ms | 80.894ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c512) | 292803 | 29421 | 17.333ms | 136.095ms |
| Uvicorn H11 (c64) | 95223 | 9522 | 6.711ms | 18.542ms |
| Uvicorn Httptools (c128) | 209835 | 20992 | 6.089ms | 17.017ms |
| Hypercorn (c128) | 65394 | 6545 | 19.497ms | 22.547ms |


### Long I/O

Plain text response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 331681 | 33343 | 15.315ms | 101.842ms |
| Granian Rsgi 100ms (c512) | 50207 | 5038 | 101.056ms | 114.274ms |
| Granian Asgi 10ms (c512) | 349233 | 35065 | 14.57ms | 63.257ms |
| Granian Asgi 100ms (c512) | 49966 | 5014 | 101.438ms | 104.839ms |
| Granian Wsgi 10ms (c256) | 207727 | 20806 | 12.279ms | 27.688ms |
| Granian Wsgi 100ms (c512) | 50678 | 5087 | 100.265ms | 115.06ms |
| Uvicorn Httptools 10ms (c512) | 308433 | 30942 | 16.507ms | 212.446ms |
| Uvicorn Httptools 100ms (c512) | 50332 | 5048 | 100.755ms | 108.27ms |
| Hypercorn 10ms (c128) | 65436 | 6549 | 19.504ms | 166.867ms |
| Hypercorn 100ms (c128) | 64620 | 6467 | 19.746ms | 162.069ms |
| Gunicorn Gevent 10ms (c128) | 85982 | 8602 | 14.856ms | 25.559ms |
| Gunicorn Gevent 100ms (c512) | 50265 | 5049 | 100.867ms | 141.459ms |
| Uwsgi 10ms (c256) | 164989 | 16549 | 10.354ms | 6686.564ms |
| Uwsgi 100ms (c128) | 164603 | 16478 | 7.666ms | 2065.555ms |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

| Clients | Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- | --- |
| 8 | Granian Rsgi | 1068470 | 202026 | 227279 |
| 8 | Granian Asgi | 983804 | 196307 | 220845 |
| 8 | Uvicorn H11 | 613566 | 124879 | 140489 |
| 8 | Hypercorn | 573682 | 91837 | 103316 |
| 16 | Granian Rsgi | 1961358 | 221428 | 235267 |
| 16 | Granian Asgi | 1935900 | 217683 | 231288 |
| 16 | Uvicorn H11 | 1257534 | 129144 | 137216 |
| 16 | Hypercorn | 1255036 | 98422 | 104573 |
| 32 | Granian Rsgi | 3556441 | 225597 | 232647 |
| 32 | Granian Asgi | 3420651 | 222222 | 229166 |
| 32 | Uvicorn H11 | N/A | N/A | N/A |
| 32 | Hypercorn | N/A | N/A | N/A |

