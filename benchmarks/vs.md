# Granian benchmarks



## VS 3rd party comparison

Run at: Tue 07 Apr 2026, 11:34    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.12.77 (CPUs: 16)    
Python version: 3.13    
Granian version: 2.7.3

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
| Granian Asgi get 10KB (c128) | 1267010 | 126671 | 1.005ms | 83.455ms |
| Granian Asgi echo 10KB (iter) (c128) | 585900 | 58584 | 2.178ms | 53.71ms |
| Gunicorn Asgi get 10KB (c128) | 367488 | 36751 | 3.468ms | 122.756ms |
| Gunicorn Asgi echo 10KB (iter) (c128) | 343532 | 34353 | 3.715ms | 135.016ms |
| Uvicorn H11 get 10KB (c128) | 128763 | 12885 | 9.903ms | 490.929ms |
| Uvicorn H11 echo 10KB (iter) (c128) | 118259 | 11835 | 10.784ms | 575.711ms |
| Uvicorn Httptools get 10KB (c128) | 512549 | 51252 | 2.488ms | 84.977ms |
| Uvicorn Httptools echo 10KB (iter) (c128) | 487358 | 48730 | 2.614ms | 99.153ms |
| Hypercorn get 10KB (c128) | 92710 | 9281 | 13.747ms | 179.906ms |
| Hypercorn echo 10KB (iter) (c128) | 80989 | 8110 | 15.734ms | 198.318ms |


### WSGI

Granian is run with `--blocking-threads 1`.

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi get 10KB (c64) | 1279479 | 127898 | 0.499ms | 19.542ms |
| Granian Wsgi echo 10KB (iter) (c64) | 1110482 | 110990 | 0.574ms | 33.503ms |
| Gunicorn Gthread get 10KB (c64) | 99884 | 9993 | 6.397ms | 25.855ms |
| Gunicorn Gthread echo 10KB (iter) (c64) | 73950 | 7400 | 8.637ms | 32.17ms |
| Gunicorn Gevent get 10KB (c64) | 140171 | 14019 | 4.382ms | 1330.088ms |
| Gunicorn Gevent echo 10KB (iter) (c64) | 90038 | 9007 | 6.714ms | 5320.485ms |
| Uwsgi get 10KB (c64) | 124526 | 12456 | 5.129ms | 29.384ms |
| Uwsgi echo 10KB (iter) (c64) | 96858 | 9689 | 6.594ms | 34.848ms |


### HTTP/2

Granian is run with `--http 2` and `--runtime-threads 2`.

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c128) | 1002316 | 100234 | 5.083ms | 12.808ms |
| Granian Asgi echo 10KB (iter) (c128) | 473569 | 47383 | 10.712ms | 19.737ms |
| Hypercorn get 10KB (c128) | 71890 | 7237 | 70.633ms | 1281.351ms |
| Hypercorn echo 10KB (iter) (c128) | 55700 | 5618 | 91.132ms | 1076.041ms |


### ASGI file responses

The benchmark performs an HTTP GET request returning a ~50KB JPEG image. While on *pathsend* the implementation is entirely provided by the underlying protocol, in all the other cases the entirety of the file is read and collected in memory and thus returned as a single byte string.

Granian is run with `--runtime-blocking-threads 1`.

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c128) | 468490 | 46847 | 2.716ms | 86.854ms |
| Gunicorn Asgi (c128) | 158541 | 15860 | 8.047ms | 307.402ms |
| Uvicorn H11 (c128) | 94017 | 9411 | 13.571ms | 451.567ms |
| Uvicorn Httptools (c128) | 184182 | 18424 | 6.929ms | 332.375ms |
| Hypercorn (c128) | 70650 | 7075 | 18.032ms | 264.404ms |


### Long I/O

Plain text response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 465569 | 46588 | 10.929ms | 111.883ms |
| Granian Rsgi 100ms (c512) | 50022 | 5050 | 101.238ms | 214.227ms |
| Granian Asgi 10ms (c512) | 452674 | 45296 | 11.237ms | 113.972ms |
| Granian Asgi 100ms (c512) | 49709 | 5019 | 101.776ms | 198.683ms |
| Granian Wsgi 10ms (c512) | 252202 | 25259 | 20.129ms | 132.759ms |
| Granian Wsgi 100ms (c512) | 26068 | 2657 | 192.814ms | 327.33ms |
| Gunicorn Asgi 10ms (c512) | 415518 | 41584 | 12.237ms | 1286.328ms |
| Gunicorn Asgi 100ms (c512) | 48679 | 4917 | 103.936ms | 1334.131ms |
| Uvicorn Httptools 10ms (c512) | 438773 | 43906 | 11.596ms | 115.006ms |
| Uvicorn Httptools 100ms (c512) | 50006 | 5049 | 101.33ms | 188.6ms |
| Hypercorn 10ms (c512) | 89997 | 9047 | 56.322ms | 1779.774ms |
| Hypercorn 100ms (c512) | 49365 | 4985 | 102.348ms | 244.405ms |
| Gunicorn Gevent 10ms (c512) | 122947 | 12339 | 41.277ms | 183.908ms |
| Gunicorn Gevent 100ms (c512) | 49693 | 5018 | 101.416ms | 275.473ms |
| Uwsgi 10ms (c512) | 978 | 413 | 989.141ms | 1113.942ms |
| Uwsgi 100ms (c512) | 0 | 0 | N/A | N/A |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

Granian is run with `--ws`.

| Clients | Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- | --- |
| 8 | Granian Rsgi | 1237948 | 270130 | 303897 |
| 8 | Granian Asgi | 1232730 | 250354 | 281648 |
| 8 | Uvicorn H11 | 863515 | 209364 | 235534 |
| 8 | Hypercorn | 767098 | 154307 | 173595 |
| 16 | Granian Rsgi | 2440579 | 328333 | 348854 |
| 16 | Granian Asgi | 2362271 | 297998 | 316623 |
| 16 | Uvicorn H11 | 1670907 | 213915 | 227285 |
| 16 | Hypercorn | 1592275 | 164696 | 174990 |
| 32 | Granian Rsgi | 4606544 | 311080 | 320802 |
| 32 | Granian Asgi | 4284942 | 295927 | 305175 |
| 32 | Uvicorn H11 | 4049752 | 209837 | 216394 |
| 32 | Hypercorn | 4032787 | 170646 | 175979 |
