# Granian benchmarks



## VS 3rd party comparison

Run at: Wed 05 Aug 2026, 16:51    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.18.41 (CPUs: 16)    
Python version: 3.13    
Granian version: 2.8.1

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
| Granian Asgi get 10KB (c128) | 1255606 | 125539 | 1.016ms | 59.463ms |
| Granian Asgi echo 10KB (iter) (c128) | 631911 | 63181 | 2.02ms | 42.666ms |
| Gunicorn Asgi get 10KB (c128) | 345317 | 34532 | 3.693ms | 190.026ms |
| Gunicorn Asgi echo 10KB (iter) (c128) | 352746 | 35275 | 3.618ms | 166.031ms |
| Uvicorn H11 get 10KB (c128) | 144928 | 14501 | 8.791ms | 372.552ms |
| Uvicorn H11 echo 10KB (iter) (c128) | 124879 | 12497 | 10.218ms | 454.955ms |
| Uvicorn Httptools get 10KB (c128) | 510514 | 51051 | 2.499ms | 123.612ms |
| Uvicorn Httptools echo 10KB (iter) (c128) | 506707 | 50665 | 2.519ms | 118.067ms |
| Hypercorn get 10KB (c128) | 93081 | 9318 | 13.718ms | 213.88ms |
| Hypercorn echo 10KB (iter) (c128) | 82045 | 8215 | 15.56ms | 197.992ms |


### WSGI

Granian is run with `--blocking-threads 1`.

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi get 10KB (c64) | 1258541 | 125824 | 0.507ms | 25.507ms |
| Granian Wsgi echo 10KB (iter) (c64) | 1081157 | 108084 | 0.589ms | 34.131ms |
| Gunicorn Gthread get 10KB (c64) | 91704 | 9175 | 6.963ms | 39.026ms |
| Gunicorn Gthread echo 10KB (iter) (c64) | 67395 | 6744 | 9.479ms | 27.452ms |
| Gunicorn Gevent get 10KB (c64) | 117588 | 11762 | 5.427ms | 352.505ms |
| Gunicorn Gevent echo 10KB (iter) (c64) | 82634 | 8268 | 7.203ms | 1190.073ms |
| Uwsgi get 10KB (c64) | 122002 | 12204 | 5.233ms | 36.54ms |
| Uwsgi echo 10KB (iter) (c64) | 93322 | 9336 | 6.842ms | 37.711ms |


### HTTP/2

Granian is run with `--http 2` and `--runtime-threads 2`.

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi get 10KB (c128) | 1297565 | 129750 | 3.924ms | 11.067ms |
| Granian Asgi echo 10KB (iter) (c128) | 622840 | 62312 | 8.176ms | 16.301ms |
| Hypercorn get 10KB (c128) | 72800 | 7327 | 69.807ms | 1387.409ms |
| Hypercorn echo 10KB (iter) (c128) | 57840 | 5833 | 87.699ms | 1006.597ms |


### ASGI file responses

The benchmark performs an HTTP GET request returning a ~50KB JPEG image. While on *pathsend* the implementation is entirely provided by the underlying protocol, in all the other cases the entirety of the file is read and collected in memory and thus returned as a single byte string.

Granian is run with `--runtime-blocking-threads 1`.

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c128) | 480486 | 48049 | 2.656ms | 46.809ms |
| Gunicorn Asgi (c128) | 162316 | 16238 | 7.852ms | 423.203ms |
| Uvicorn H11 (c128) | 96789 | 9689 | 13.176ms | 786.087ms |
| Uvicorn Httptools (c128) | 299968 | 30004 | 4.255ms | 208.454ms |
| Hypercorn (c128) | 78214 | 7832 | 16.322ms | 256.01ms |


### Long I/O

Plain text response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 462588 | 46284 | 10.988ms | 119.662ms |
| Granian Rsgi 100ms (c512) | 49670 | 5015 | 101.566ms | 204.571ms |
| Granian Asgi 10ms (c512) | 458393 | 45859 | 11.079ms | 131.807ms |
| Granian Asgi 100ms (c512) | 50176 | 5067 | 101.223ms | 193.416ms |
| Granian Wsgi 10ms (c512) | 431948 | 43224 | 11.746ms | 139.536ms |
| Granian Wsgi 100ms (c512) | 50176 | 5066 | 100.978ms | 203.544ms |
| Gunicorn Asgi 10ms (c512) | 382431 | 38275 | 13.301ms | 1281.082ms |
| Gunicorn Asgi 100ms (c512) | 48493 | 4898 | 104.476ms | 1327.489ms |
| Uvicorn Httptools 10ms (c512) | 438800 | 43903 | 11.606ms | 117.455ms |
| Uvicorn Httptools 100ms (c512) | 50049 | 5053 | 101.145ms | 219.915ms |
| Hypercorn 10ms (c512) | 88068 | 8854 | 57.654ms | 1786.093ms |
| Hypercorn 100ms (c512) | 49295 | 4977 | 102.583ms | 248.031ms |
| Gunicorn Gevent 10ms (c512) | 109564 | 11002 | 46.286ms | 177.549ms |
| Gunicorn Gevent 100ms (c512) | 49783 | 5027 | 101.48ms | 243.75ms |
| Uwsgi 10ms (c512) | 978 | 413 | 994.956ms | 2291.321ms |
| Uwsgi 100ms (c512) | 5 | 54 | 9786.627ms | 9986.837ms |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

Granian is run with `--ws` and `--runtime-threads 2`.

| Clients | Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- | --- |
| 8 | Granian Rsgi | 1246391 | 280087 | 315098 |
| 8 | Granian Asgi | 947581 | 268663 | 302245 |
| 8 | Gunicorn Asgi | 951967 | 240771 | 270868 |
| 8 | Uvicorn H11 | 773447 | 144061 | 162069 |
| 8 | Hypercorn | 734416 | 81112 | 91250 |
| 16 | Granian Rsgi | 2233185 | 336808 | 357859 |
| 16 | Granian Asgi | 2340178 | 339787 | 361024 |
| 16 | Gunicorn Asgi | 1921876 | 324652 | 344943 |
| 16 | Uvicorn H11 | 1546168 | 165756 | 176116 |
| 16 | Hypercorn | 1433243 | 102500 | 108906 |
| 32 | Granian Rsgi | 3652066 | 330844 | 341182 |
| 32 | Granian Asgi | 3548805 | 327981 | 338230 |
| 32 | Gunicorn Asgi | 3802710 | 366978 | 378446 |
| 32 | Uvicorn H11 | 3163470 | 183706 | 189447 |
| 32 | Hypercorn | N/A | N/A | N/A |
