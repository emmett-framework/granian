# Granian benchmarks



## VS 3rd party comparison

Run at: Sun 26 May 2024, 08:49    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.4.0    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c64) | 593898 | 39599 | 1.613ms | 4.402ms |
| Granian Asgi [POST] (c128) | 352271 | 23504 | 5.426ms | 44.78ms |
| Uvicorn H11 [GET] (c64) | 111638 | 7444 | 8.579ms | 17.824ms |
| Uvicorn H11 [POST] (c128) | 105999 | 7073 | 18.038ms | 50.118ms |
| Uvicorn Httptools [GET] (c128) | 532629 | 35543 | 3.594ms | 29.382ms |
| Uvicorn Httptools [POST] (c128) | 503829 | 33610 | 3.801ms | 20.716ms |
| Hypercorn [GET] (c128) | 73076 | 4876 | 26.172ms | 47.024ms |
| Hypercorn [POST] (c128) | 66087 | 4409 | 28.939ms | 51.308ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c128) | 569654 | 38005 | 3.357ms | 41.008ms |
| Granian Wsgi [POST] (c64) | 541985 | 36137 | 1.768ms | 4.964ms |
| Gunicorn Gthread [GET] (c64) | 54480 | 3632 | 17.578ms | 28.043ms |
| Gunicorn Gthread [POST] (c64) | 52914 | 3528 | 18.105ms | 20.571ms |
| Gunicorn Gevent [GET] (c64) | 93494 | 6234 | 8.776ms | 5057.868ms |
| Gunicorn Gevent [POST] (c256) | 88608 | 5920 | 26.171ms | 13009.009ms |
| Uwsgi [GET] (c128) | 107072 | 7144 | 17.774ms | 2076.566ms |
| Uwsgi [POST] (c128) | 106491 | 7105 | 17.864ms | 3092.402ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c64) | 538453 | 35901 | 1.78ms | 5.107ms |
| Granian Asgi [POST] (c64) | 321402 | 21429 | 2.982ms | 8.908ms |
| Hypercorn [GET] (c128) | 22851 | 1525 | 83.162ms | 311.394ms |
| Hypercorn [POST] (c64) | 41742 | 2783 | 22.954ms | 54.771ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c64) | 400964 | 26734 | 2.39ms | 6.461ms |
| Uvicorn H11 (c128) | 118766 | 7922 | 16.117ms | 39.497ms |
| Uvicorn Httptools (c128) | 300661 | 20061 | 6.367ms | 20.633ms |
| Hypercorn (c128) | 73177 | 4883 | 26.13ms | 53.916ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 581493 | 38909 | 13.114ms | 122.726ms |
| Granian Rsgi 100ms (c512) | 75508 | 5053 | 100.651ms | 159.396ms |
| Granian Asgi 10ms (c512) | 565313 | 37811 | 13.492ms | 118.059ms |
| Granian Asgi 100ms (c512) | 75553 | 5053 | 100.762ms | 156.423ms |
| Granian Wsgi 10ms (c128) | 169691 | 11323 | 11.273ms | 35.884ms |
| Granian Wsgi 100ms (c512) | 75818 | 5072 | 100.311ms | 154.878ms |
| Uvicorn Httptools 10ms (c512) | 325474 | 21800 | 23.382ms | 131.764ms |
| Uvicorn Httptools 100ms (c512) | 75267 | 5039 | 100.878ms | 215.692ms |
| Hypercorn 10ms (c128) | 72245 | 4821 | 26.466ms | 33.802ms |
| Hypercorn 100ms (c128) | 73447 | 4901 | 26.032ms | 33.85ms |
| Gunicorn Gevent 10ms (c64) | 85234 | 5683 | 11.242ms | 20.871ms |
| Gunicorn Gevent 100ms (c512) | 72789 | 4871 | 104.473ms | 168.753ms |
| Uwsgi 10ms (c128) | 106872 | 7133 | 17.771ms | 2072.306ms |
| Uwsgi 100ms (c512) | 107188 | 7171 | 64.429ms | 7571.134ms |

