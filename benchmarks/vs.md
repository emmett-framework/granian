# Granian benchmarks



## VS 3rd party comparison

Run at: Mon 01 Dec 2025, 16:21    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.12.58 (CPUs: 16)    
Python version: 3.13    
Granian version: 2.6.0

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
| Granian Asgi get 10KB (c128) | 1120599 | 112038 | 1.138ms | 47.111ms |
| Granian Asgi echo 10KB (iter) (c128) | 495957 | 49591 | 2.57ms | 74.4ms |
| Uvicorn H11 get 10KB (c128) | 92608 | 9271 | 13.773ms | 661.365ms |
| Uvicorn H11 echo 10KB (iter) (c128) | 79095 | 7920 | 16.116ms | 852.353ms |
| Uvicorn Httptools get 10KB (c128) | 372015 | 37202 | 3.431ms | 163.172ms |
| Uvicorn Httptools echo 10KB (iter) (c128) | 326370 | 32640 | 3.908ms | 187.28ms |
| Hypercorn get 10KB (c128) | 63625 | 6374 | 20.054ms | 302.944ms |
| Hypercorn echo 10KB (iter) (c128) | 56003 | 5611 | 22.729ms | 243.464ms |


### WSGI

Granian is run with `--blocking-threads 1`.

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi get 10KB (c64) | 1258206 | 125767 | 0.507ms | 24.521ms |
| Granian Wsgi echo 10KB (iter) (c64) | 968919 | 96869 | 0.658ms | 33.514ms |
| Gunicorn Gthread get 10KB (c64) | 64957 | 6501 | 9.826ms | 47.665ms |
| Gunicorn Gthread echo 10KB (iter) (c64) | 48710 | 4876 | 13.098ms | 56.932ms |
| Gunicorn Gevent get 10KB (c64) | 92780 | 9282 | 6.109ms | 3698.103ms |
| Gunicorn Gevent echo 10KB (iter) (c64) | 66363 | 6641 | 1.224ms | 7632.984ms |
| Uwsgi get 10KB (c64) | 125203 | 12524 | 5.1ms | 32.23ms |
| Uwsgi echo 10KB (iter) (c64) | 96651 | 9670 | 6.61ms | 26.501ms |


### HTTP/2

Granian is run with `--http 2` and `--runtime-threads 2`.

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c128) | 727258 | 72742 | 7.007ms | 20.31ms |
| Granian Asgi echo 10KB (iter) (c128) | 358296 | 35869 | 14.22ms | 27.098ms |
| Hypercorn get 10KB (c128) | 47195 | 4768 | 107.262ms | 1947.351ms |
| Hypercorn echo 10KB (iter) (c128) | 38204 | 3870 | 132.407ms | 1570.028ms |


### ASGI file responses

The benchmark performs an HTTP GET request returning a ~50KB JPEG image. While on *pathsend* the implementation is entirely provided by the underlying protocol, in all the other cases the entirety of the file is read and collected in memory and thus returned as a single byte string.

Granian is run with `--runtime-blocking-threads 1`.

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c128) | 470582 | 47055 | 2.71ms | 68.904ms |
| Uvicorn H11 (c128) | 67693 | 6780 | 18.801ms | 1041.191ms |
| Uvicorn Httptools (c128) | 166445 | 16652 | 7.669ms | 394.407ms |
| Hypercorn (c128) | 52310 | 5242 | 24.365ms | 356.905ms |


### Long I/O

Plain text response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 455101 | 45537 | 11.14ms | 146.055ms |
| Granian Rsgi 100ms (c512) | 50082 | 5057 | 101.309ms | 215.883ms |
| Granian Asgi 10ms (c512) | 432970 | 43327 | 11.742ms | 125.013ms |
| Granian Asgi 100ms (c512) | 49393 | 4988 | 102.282ms | 240.509ms |
| Granian Wsgi 10ms (c512) | 403305 | 40359 | 12.611ms | 117.238ms |
| Granian Wsgi 100ms (c512) | 50176 | 5065 | 100.86ms | 192.847ms |
| Uvicorn Httptools 10ms (c512) | 342402 | 34272 | 14.81ms | 143.097ms |
| Uvicorn Httptools 100ms (c512) | 49856 | 5034 | 101.336ms | 238.871ms |
| Hypercorn 10ms (c512) | 60094 | 6057 | 84.263ms | 3228.749ms |
| Hypercorn 100ms (c512) | 49229 | 4970 | 102.755ms | 233.677ms |
| Gunicorn Gevent 10ms (c512) | 80753 | 8122 | 62.714ms | 226.663ms |
| Gunicorn Gevent 100ms (c512) | 49796 | 5029 | 101.464ms | 277.197ms |
| Uwsgi 10ms (c512) | 934 | 412 | 1037.048ms | 1899.769ms |
| Uwsgi 100ms (c512) | 0 | 0 | N/A | N/A |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

Granian is run with `--ws`.

| Clients | Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- | --- |
| 8 | Granian Rsgi | 978859 | 216753 | 243847 |
| 8 | Granian Asgi | 1017772 | 213299 | 239961 |
| 8 | Uvicorn H11 | 633314 | 125949 | 141692 |
| 8 | Hypercorn | 587286 | 91568 | 103014 |
| 16 | Granian Rsgi | 2026042 | 232389 | 246914 |
| 16 | Granian Asgi | 1979518 | 231791 | 246278 |
| 16 | Uvicorn H11 | 1317826 | 129333 | 137416 |
| 16 | Hypercorn | 1281565 | 99381 | 105592 |
| 32 | Granian Rsgi | 3717750 | 232016 | 239267 |
| 32 | Granian Asgi | 3614998 | 232219 | 239475 |
| 32 | Uvicorn H11 | N/A | N/A | N/A |
| 32 | Hypercorn | N/A | N/A | N/A |

