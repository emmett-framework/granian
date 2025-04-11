# Granian benchmarks



## VS 3rd party comparison

Run at: Fri 11 Apr 2025, 12:46    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Python version: 3.12    
Granian version: 2.2.4

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c256) | 1175387 | 117954 | 2.168ms | 28.209ms |
| Granian Asgi echo 10KB (iter) (c256) | 588711 | 59035 | 4.331ms | 32.221ms |
| Uvicorn H11 get 10KB (c128) | 95536 | 9565 | 13.343ms | 18.028ms |
| Uvicorn H11 echo 10KB (iter) (c64) | 83158 | 8317 | 7.675ms | 17.894ms |
| Uvicorn Httptools get 10KB (c128) | 402494 | 40281 | 3.173ms | 19.949ms |
| Uvicorn Httptools echo 10KB (iter) (c128) | 348414 | 34867 | 3.664ms | 17.421ms |
| Hypercorn get 10KB (c128) | 65152 | 6525 | 19.564ms | 182.485ms |
| Hypercorn echo 10KB (iter) (c128) | 57793 | 5783 | 22.086ms | 217.672ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi get 10KB (c64) | 1196539 | 119671 | 0.533ms | 2.685ms |
| Granian Wsgi echo 10KB (iter) (c64) | 859286 | 85928 | 0.744ms | 3.725ms |
| Gunicorn Gthread get 10KB (c64) | 61623 | 6163 | 10.364ms | 27.021ms |
| Gunicorn Gthread echo 10KB (iter) (c64) | 45960 | 4596 | 13.892ms | 28.653ms |
| Gunicorn Gevent get 10KB (c64) | 93803 | 9381 | 4.137ms | 5171.038ms |
| Gunicorn Gevent echo 10KB (iter) (c64) | 63209 | 6321 | 4.926ms | 8053.5ms |
| Uwsgi get 10KB (c128) | 166344 | 16660 | 7.608ms | 6619.882ms |
| Uwsgi echo 10KB (iter) (c256) | 192803 | 19347 | 8.642ms | 6918.115ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c64) | 671246 | 67127 | 0.952ms | 3.217ms |
| Granian Asgi echo 10KB (iter) (c512) | 122197 | 12279 | 41.52ms | 143.346ms |
| Hypercorn get 10KB (c128) | 41077 | 4112 | 31.045ms | 339.642ms |
| Hypercorn echo 10KB (iter) (c128) | 31379 | 3139 | 40.614ms | 82.642ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c128) | 493044 | 49341 | 2.591ms | 30.045ms |
| Uvicorn H11 (c128) | 95534 | 9561 | 13.361ms | 96.573ms |
| Uvicorn Httptools (c64) | 159326 | 15932 | 4.01ms | 10.124ms |
| Hypercorn (c128) | 65744 | 6580 | 19.413ms | 200.78ms |


### Long I/O

Plain text response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 472474 | 47379 | 10.791ms | 46.135ms |
| Granian Rsgi 100ms (c512) | 50176 | 5038 | 100.97ms | 104.07ms |
| Granian Asgi 10ms (c512) | 459751 | 46235 | 11.05ms | 66.322ms |
| Granian Asgi 100ms (c512) | 50176 | 5034 | 101.163ms | 105.142ms |
| Granian Wsgi 10ms (c512) | 409851 | 41125 | 12.431ms | 38.178ms |
| Granian Wsgi 100ms (c512) | 50566 | 5069 | 100.502ms | 105.142ms |
| Uvicorn Httptools 10ms (c512) | 309499 | 31071 | 16.442ms | 44.496ms |
| Uvicorn Httptools 100ms (c512) | 50273 | 5049 | 100.726ms | 106.954ms |
| Hypercorn 10ms (c128) | 65150 | 6519 | 19.579ms | 33.791ms |
| Hypercorn 100ms (c128) | 65366 | 6540 | 19.536ms | 182.295ms |
| Gunicorn Gevent 10ms (c256) | 85365 | 8559 | 29.826ms | 68.292ms |
| Gunicorn Gevent 100ms (c512) | 50184 | 5041 | 100.786ms | 157.731ms |
| Uwsgi 10ms (c256) | 166182 | 16645 | 14.315ms | 7635.042ms |
| Uwsgi 100ms (c64) | 166253 | 16627 | 3.842ms | 7.038ms |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

| Clients | Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- | --- |
| 8 | Granian Rsgi | 1014543 | 213947 | 240691 |
| 8 | Granian Asgi | 1028839 | 211641 | 238096 |
| 8 | Uvicorn H11 | 637261 | 126155 | 141924 |
| 8 | Hypercorn | 565863 | 93121 | 104761 |
| 16 | Granian Rsgi | 1958485 | 226011 | 240137 |
| 16 | Granian Asgi | 1968704 | 226035 | 240162 |
| 16 | Uvicorn H11 | 1233808 | 130398 | 138548 |
| 16 | Hypercorn | 1239932 | 99423 | 105637 |
| 32 | Granian Rsgi | 3629573 | 220310 | 227195 |
| 32 | Granian Asgi | 3544010 | 225932 | 232992 |
| 32 | Uvicorn H11 | N/A | N/A | N/A |
| 32 | Hypercorn | N/A | N/A | N/A |

