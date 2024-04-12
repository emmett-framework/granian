
# Granian benchmarks

## VS 3rd party comparison

Run at: Thu 11 Apr 2024, 23:57
Environment: GHA (CPUs: 4)
Python version: 3.11
Granian version: 1.2.2

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c64) | 644144 | 42945.04707423531 | 1.488ms | 4.011ms |
| Granian Asgi [POST] (c32) | 4717 | 314.44034360774185 | 101.561ms | 722.927ms |
| Uvicorn H11 [GET] (c64) | 115668 | 7712.042880014528 | 8.285ms | 22.951ms |
| Uvicorn H11 [POST] (c64) | 103514 | 6901.67809203591 | 9.26ms | 23.888ms |
| Uvicorn Httptools [GET] (c64) | 528680 | 35249.16265666782 | 1.813ms | 5.71ms |
| Uvicorn Httptools [POST] (c128) | 495777 | 33082.58324666878 | 3.857ms | 36.519ms |
| Hypercorn [GET] (c128) | 72087 | 4808.6859049554805 | 26.525ms | 33.689ms |
| Hypercorn [POST] (c128) | 66095 | 4409.692740155614 | 28.937ms | 48.532ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c128) | 603486 | 40266.184518005255 | 3.17ms | 28.627ms |
| Granian Wsgi [POST] (c256) | 549415 | 36693.83260890369 | 6.955ms | 52.467ms |
| Gunicorn Gthread [GET] (c32) | 60244 | 4016.205643634876 | 7.964ms | 16.569ms |
| Gunicorn Gthread [POST] (c32) | 56630 | 3775.259407462036 | 8.472ms | 12.292ms |
| Gunicorn Gevent [GET] (c32) | 94250 | 6283.221909608159 | 4.9ms | 2732.202ms |
| Gunicorn Gevent [POST] (c128) | 89549 | 5974.175995977868 | 17.813ms | 6416.631ms |
| Uwsgi [GET] (c32) | 107918 | 7194.3124789715075 | 4.446ms | 15.004ms |
| Uwsgi [POST] (c32) | 108131 | 7208.403525324598 | 4.437ms | 11.456ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c64) | 540518 | 36040.0926017107 | 1.773ms | 9.625ms |
| Granian Asgi [POST] (c64) | 1292 | 86.14193726468368 | 727.082ms | 3354.925ms |
| Hypercorn [GET] (c64) | 22049 | 1470.1523830000854 | 43.352ms | 174.618ms |
| Hypercorn [POST] (c64) | 40320 | 2688.4030229778464 | 23.742ms | 52.401ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (c128) | 225461 | 15041.321237225471 | 8.489ms | 29.073ms |
| Granian pathsend (c256) | 0 | 0 | N/A | N/A |
| Uvicorn H11 (c64) | 118237 | 7883.2728579743125 | 8.105ms | 19.082ms |
| Uvicorn Httptools (c128) | 308366 | 20573.293314188744 | 6.209ms | 21.775ms |
| Hypercorn (c128) | 72352 | 4826.861714831045 | 26.447ms | 41.253ms |

