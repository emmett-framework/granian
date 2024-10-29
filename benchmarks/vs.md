# Granian benchmarks



## VS 3rd party comparison

Run at: Mon 28 Oct 2024, 02:10    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.6.2    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c64) | 412041 | 41213 | 1.549ms | 4.255ms |
| Granian Asgi [POST] (c64) | 221079 | 22111 | 2.888ms | 5.488ms |
| Uvicorn H11 [GET] (c64) | 75123 | 7513 | 8.5ms | 22.009ms |
| Uvicorn H11 [POST] (c64) | 66085 | 6609 | 9.657ms | 26.091ms |
| Uvicorn Httptools [GET] (c128) | 339905 | 34045 | 3.745ms | 23.819ms |
| Uvicorn Httptools [POST] (c128) | 314202 | 31455 | 4.055ms | 23.736ms |
| Hypercorn [GET] (c128) | 48884 | 4893 | 26.054ms | 42.096ms |
| Hypercorn [POST] (c128) | 42517 | 4257 | 29.922ms | 54.175ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c256) | 381571 | 38324 | 6.644ms | 70.442ms |
| Granian Wsgi [POST] (c128) | 347832 | 34819 | 3.662ms | 34.109ms |
| Gunicorn Gthread [GET] (c64) | 36680 | 3669 | 17.386ms | 19.742ms |
| Gunicorn Gthread [POST] (c64) | 35078 | 3508 | 18.191ms | 20.25ms |
| Gunicorn Gevent [GET] (c128) | 62624 | 6270 | 7.909ms | 9876.337ms |
| Gunicorn Gevent [POST] (c128) | 57223 | 5729 | 12.683ms | 9777.452ms |
| Uwsgi [GET] (c64) | 71314 | 7131 | 8.955ms | 16.786ms |
| Uwsgi [POST] (c64) | 71214 | 7122 | 8.964ms | 13.792ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c128) | 346031 | 34673 | 3.673ms | 44.303ms |
| Granian Asgi [POST] (c64) | 196285 | 19633 | 3.25ms | 10.419ms |
| Hypercorn [GET] (c64) | 30507 | 3051 | 20.914ms | 50.49ms |
| Hypercorn [POST] (c64) | 27155 | 2716 | 23.491ms | 56.879ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c64) | 296068 | 29608 | 2.157ms | 4.387ms |
| Uvicorn H11 (c128) | 74931 | 7505 | 16.979ms | 43.618ms |
| Uvicorn Httptools (c64) | 190766 | 19080 | 3.347ms | 7.536ms |
| Hypercorn (c128) | 47773 | 4782 | 26.646ms | 44.924ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 398741 | 40076 | 12.704ms | 122.917ms |
| Granian Rsgi 100ms (c512) | 50074 | 5035 | 100.659ms | 161.13ms |
| Granian Asgi 10ms (c512) | 368346 | 37047 | 13.753ms | 126.133ms |
| Granian Asgi 100ms (c512) | 50056 | 5031 | 100.724ms | 151.241ms |
| Granian Wsgi 10ms (c128) | 109932 | 11005 | 11.578ms | 34.242ms |
| Granian Wsgi 100ms (c512) | 50218 | 5050 | 100.365ms | 162.437ms |
| Uvicorn Httptools 10ms (c256) | 227853 | 22854 | 11.14ms | 67.594ms |
| Uvicorn Httptools 100ms (c512) | 50074 | 5028 | 100.869ms | 181.026ms |
| Hypercorn 10ms (c128) | 48782 | 4884 | 26.081ms | 46.542ms |
| Hypercorn 100ms (c128) | 49325 | 4938 | 25.812ms | 46.076ms |
| Gunicorn Gevent 10ms (c128) | 55121 | 5519 | 23.096ms | 60.147ms |
| Gunicorn Gevent 100ms (c512) | 47896 | 4812 | 105.287ms | 191.957ms |
| Uwsgi 10ms (c256) | 71141 | 7136 | 34.26ms | 3345.351ms |
| Uwsgi 100ms (c256) | 70790 | 7102 | 34.882ms | 3071.967ms |


### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

| Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- |
| Granian Rsgi (c16) | 962877 | 76296 | 81065 |
| Granian Asgi (c16) | 941480 | 73632 | 78234 |
| Uvicorn H11 (c8) | 456731 | 80133 | 90149 |

