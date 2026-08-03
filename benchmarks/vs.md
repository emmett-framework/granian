# Granian benchmarks



## VS 3rd party comparison

Run at: Mon 03 Aug 2026, 12:45    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.18.41 (CPUs: 16)    
Python version: 3.13    
Granian version: 2.8.0

### Methodology

Unless otherwise specified in the specific benchmark section, Granian is run:

- Using default configuration, thus:
  - 1 worker
  - 1 runtime thread
- With `--runtime-mode` set to `st` on ASGI and `mt` otherwise
- With `--http 1` flag
- With `--no-ws` flag

Tests are peformed using `oha` utility, with the concurrency specified in the specific test. The test run for 10 seconds, preceeded by a *primer* run at concurrency 8 for 4 seconds, and a *warmup* run at the maximum configured concurrency for the test for 3 seconds.

All the async servers – including Granian – are using `uvloop` for the asyncio event-loop implementation.

All the reported 3rd party servers were installed using the latest available version at the time of the run.

The *get* benchmark consists of an HTTP GET request returning a 10KB plain-text response (the response is a single static byte string).

The *echo* benchmark consists of an HTTP POST request with a 10KB plain-text body, which will be *streamed* back (the iteration happens in chunks with a dimension depending on the underlying protocol).

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c128) | 1235705 | 123537 | 1.033ms | 38.304ms |
| Granian Asgi echo 10KB (iter) (c128) | 556215 | 55620 | 2.292ms | 57.006ms |
| Gunicorn Asgi get 10KB (c128) | 362929 | 36296 | 3.514ms | 129.466ms |
| Gunicorn Asgi echo 10KB (iter) (c128) | 349038 | 34906 | 3.658ms | 199.999ms |
| Uvicorn H11 get 10KB (c128) | 143359 | 14344 | 8.914ms | 479.686ms |
| Uvicorn H11 echo 10KB (iter) (c128) | 125967 | 12607 | 10.133ms | 543.992ms |
| Uvicorn Httptools get 10KB (c128) | 624437 | 62434 | 2.045ms | 72.907ms |
| Uvicorn Httptools echo 10KB (iter) (c128) | 509261 | 50921 | 2.504ms | 94.397ms |
| Hypercorn get 10KB (c128) | 94069 | 9417 | 13.567ms | 213.714ms |
| Hypercorn echo 10KB (iter) (c128) | 83516 | 8362 | 15.28ms | 186.776ms |


### WSGI

Granian is run with `--blocking-threads 1`.

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi get 10KB (c64) | 1272203 | 127189 | 0.501ms | 17.918ms |
| Granian Wsgi echo 10KB (iter) (c64) | 1100008 | 109968 | 0.58ms | 26.648ms |
| Gunicorn Gthread get 10KB (c64) | 93141 | 9319 | 6.857ms | 32.068ms |
| Gunicorn Gthread echo 10KB (iter) (c64) | 67634 | 6769 | 9.445ms | 29.247ms |
| Gunicorn Gevent get 10KB (c64) | 123908 | 12395 | 5.117ms | 178.808ms |
| Gunicorn Gevent echo 10KB (iter) (c64) | 85351 | 8540 | 7.454ms | 325.536ms |
| Uwsgi get 10KB (c64) | 124277 | 12430 | 5.138ms | 33.385ms |
| Uwsgi echo 10KB (iter) (c64) | 96384 | 9644 | 6.622ms | 41.797ms |


### HTTP/2

Granian is run with `--http 2` and `--runtime-threads 2`.

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c128) | 902115 | 90231 | 5.648ms | 13.64ms |
| Granian Asgi echo 10KB (iter) (c128) | 431920 | 43229 | 11.808ms | 22.166ms |
| Hypercorn get 10KB (c128) | 73274 | 7376 | 69.387ms | 1276.706ms |
| Hypercorn echo 10KB (iter) (c128) | 57344 | 5783 | 88.477ms | 1074.801ms |


### ASGI file responses

The benchmark performs an HTTP GET request returning a ~50KB JPEG image. While on *pathsend* the implementation is entirely provided by the underlying protocol, in all the other cases the entirety of the file is read and collected in memory and thus returned as a single byte string.

Granian is run with `--runtime-blocking-threads 1`.

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c128) | 477307 | 47730 | 2.671ms | 79.621ms |
| Gunicorn Asgi (c128) | 162200 | 16228 | 7.862ms | 499.846ms |
| Uvicorn H11 (c128) | 99069 | 9918 | 12.896ms | 729.384ms |
| Uvicorn Httptools (c128) | 209513 | 20958 | 6.091ms | 330.347ms |
| Hypercorn (c128) | 77786 | 7790 | 16.392ms | 257.645ms |


### Long I/O

Plain text response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 463164 | 46345 | 10.968ms | 126.358ms |
| Granian Rsgi 100ms (c512) | 49666 | 5015 | 101.374ms | 234.903ms |
| Granian Asgi 10ms (c512) | 456332 | 45653 | 11.149ms | 115.421ms |
| Granian Asgi 100ms (c512) | 49363 | 4985 | 102.348ms | 224.397ms |
| Granian Wsgi 10ms (c512) | 430270 | 43058 | 11.807ms | 123.666ms |
| Granian Wsgi 100ms (c512) | 50176 | 5066 | 100.995ms | 243.912ms |
| Gunicorn Asgi 10ms (c512) | 375017 | 37535 | 13.565ms | 1274.769ms |
| Gunicorn Asgi 100ms (c512) | 48683 | 4918 | 104.268ms | 1330.548ms |
| Uvicorn Httptools 10ms (c512) | 437152 | 43745 | 11.652ms | 113.99ms |
| Uvicorn Httptools 100ms (c512) | 49887 | 5036 | 101.232ms | 232.688ms |
| Hypercorn 10ms (c512) | 89288 | 8975 | 56.928ms | 1739.885ms |
| Hypercorn 100ms (c512) | 49435 | 4992 | 102.374ms | 230.898ms |
| Gunicorn Gevent 10ms (c512) | 111744 | 11221 | 45.353ms | 184.561ms |
| Gunicorn Gevent 100ms (c512) | 49734 | 5022 | 101.486ms | 254.298ms |
| Uwsgi 10ms (c512) | 960 | 440 | 1009.411ms | 1235.788ms |
| Uwsgi 100ms (c512) | 9 | 55 | 9551.08ms | 9925.832ms |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

Granian is run with `--ws`.

| Clients | Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- | --- |
| 8 | Granian Rsgi | 920295 | 269184 | 302832 |
| 8 | Granian Asgi | 930073 | 242418 | 272721 |
| 8 | Gunicorn Asgi | 963541 | 339416 | 381843 |
| 8 | Uvicorn H11 | 1022330 | 271141 | 305033 |
| 8 | Hypercorn | 759658 | 154459 | 173767 |
| 16 | Granian Rsgi | 2119699 | 304783 | 323832 |
| 16 | Granian Asgi | 1852436 | 283189 | 300889 |
| 16 | Gunicorn Asgi | 2802127 | 374715 | 398135 |
| 16 | Uvicorn H11 | 2100111 | 289150 | 307222 |
| 16 | Hypercorn | 1573633 | 165177 | 175500 |
| 32 | Granian Rsgi | 3719904 | 290081 | 299146 |
| 32 | Granian Asgi | 3674098 | 284879 | 293781 |
| 32 | Gunicorn Asgi | 5747305 | 348333 | 359219 |
| 32 | Uvicorn H11 | 4282829 | 290692 | 299777 |
| 32 | Hypercorn | 4096389 | 171830 | 177200 |

