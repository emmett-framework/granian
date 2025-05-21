# Granian benchmarks



## VS 3rd party comparison

Run at: Wed 21 May 2025, 00:21    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Python version: 3.12    
Granian version: 2.3.1

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c128) | 1209492 | 121065 | 1.056ms | 21.722ms |
| Granian Asgi echo 10KB (iter) (c256) | 604180 | 60601 | 4.218ms | 33.901ms |
| Uvicorn H11 get 10KB (c64) | 95321 | 9532 | 6.707ms | 16.222ms |
| Uvicorn H11 echo 10KB (iter) (c64) | 83145 | 8315 | 7.682ms | 22.916ms |
| Uvicorn Httptools get 10KB (c128) | 396594 | 39694 | 3.22ms | 16.505ms |
| Uvicorn Httptools echo 10KB (iter) (c128) | 349481 | 34974 | 3.652ms | 17.761ms |
| Hypercorn get 10KB (c128) | 65706 | 6575 | 19.43ms | 192.337ms |
| Hypercorn echo 10KB (iter) (c128) | 57788 | 5784 | 22.07ms | 187.016ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi get 10KB (c64) | 1234817 | 123487 | 0.517ms | 3.097ms |
| Granian Wsgi echo 10KB (iter) (c64) | 866084 | 86606 | 0.738ms | 3.67ms |
| Gunicorn Gthread get 10KB (c64) | 63449 | 6345 | 10.068ms | 13.108ms |
| Gunicorn Gthread echo 10KB (iter) (c64) | 44999 | 4500 | 14.192ms | 25.237ms |
| Gunicorn Gevent get 10KB (c64) | 94102 | 9411 | 3.541ms | 7460.271ms |
| Gunicorn Gevent echo 10KB (iter) (c64) | 63017 | 6302 | 4.813ms | 7860.646ms |
| Uwsgi get 10KB (c64) | 165575 | 16558 | 3.858ms | 5.763ms |
| Uwsgi echo 10KB (iter) (c256) | 193382 | 19386 | 10.264ms | 6588.85ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c64) | 670752 | 67079 | 0.952ms | 4.25ms |
| Granian Asgi echo 10KB (iter) (c512) | 122105 | 12274 | 41.544ms | 157.271ms |
| Hypercorn get 10KB (c128) | 41415 | 4143 | 30.809ms | 399.949ms |
| Hypercorn echo 10KB (iter) (c128) | 31932 | 3195 | 39.934ms | 82.546ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c128) | 500730 | 50116 | 2.55ms | 30.838ms |
| Uvicorn H11 (c128) | 95279 | 9533 | 13.408ms | 317.559ms |
| Uvicorn Httptools (c64) | 151960 | 15196 | 4.204ms | 10.432ms |
| Hypercorn (c128) | 65378 | 6542 | 19.515ms | 30.242ms |


### Long I/O

Plain text response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 469509 | 47111 | 10.849ms | 79.164ms |
| Granian Rsgi 100ms (c512) | 50291 | 5054 | 100.853ms | 104.491ms |
| Granian Asgi 10ms (c512) | 463502 | 46495 | 10.99ms | 61.469ms |
| Granian Asgi 100ms (c512) | 50284 | 5050 | 101.0ms | 104.911ms |
| Granian Wsgi 10ms (c512) | 418746 | 42024 | 12.165ms | 47.537ms |
| Granian Wsgi 100ms (c512) | 50294 | 5051 | 100.391ms | 104.889ms |
| Uvicorn Httptools 10ms (c512) | 310265 | 31111 | 16.425ms | 153.543ms |
| Uvicorn Httptools 100ms (c512) | 50276 | 5046 | 100.763ms | 108.566ms |
| Hypercorn 10ms (c128) | 64491 | 6456 | 19.781ms | 194.227ms |
| Hypercorn 100ms (c128) | 64360 | 6441 | 19.829ms | 190.223ms |
| Gunicorn Gevent 10ms (c128) | 85902 | 8598 | 14.86ms | 26.707ms |
| Gunicorn Gevent 100ms (c512) | 50288 | 5052 | 100.758ms | 138.529ms |
| Uwsgi 10ms (c512) | 163394 | 16412 | 20.75ms | 7610.089ms |
| Uwsgi 100ms (c128) | 164639 | 16474 | 7.696ms | 3354.149ms |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

| Clients | Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- | --- |
| 8 | Granian Rsgi | 1022244 | 213279 | 239939 |
| 8 | Granian Asgi | 1029782 | 216303 | 243341 |
| 8 | Uvicorn H11 | 610660 | 125721 | 141436 |
| 8 | Hypercorn | 580836 | 90975 | 102347 |
| 16 | Granian Rsgi | 1937499 | 226845 | 241023 |
| 16 | Granian Asgi | 1935757 | 226807 | 240983 |
| 16 | Uvicorn H11 | 1261027 | 128309 | 136329 |
| 16 | Hypercorn | 1248253 | 98296 | 104439 |
| 32 | Granian Rsgi | 3572750 | 225997 | 233059 |
| 32 | Granian Asgi | 3597453 | 225285 | 232325 |
| 32 | Uvicorn H11 | N/A | N/A | N/A |
| 32 | Hypercorn | N/A | N/A | N/A |

