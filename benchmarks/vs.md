# Granian benchmarks



## VS 3rd party comparison

Run at: Sun 07 Jul 2024, 16:41    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.5.0    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c256) | 430989 | 43263 | 5.887ms | 77.225ms |
| Granian Asgi [POST] (c128) | 220124 | 22038 | 5.788ms | 29.382ms |
| Uvicorn H11 [GET] (c128) | 78906 | 7899 | 16.15ms | 27.199ms |
| Uvicorn H11 [POST] (c64) | 69602 | 6962 | 9.169ms | 20.795ms |
| Uvicorn Httptools [GET] (c128) | 370778 | 37121 | 3.438ms | 18.315ms |
| Uvicorn Httptools [POST] (c128) | 337939 | 33833 | 3.772ms | 18.323ms |
| Hypercorn [GET] (c128) | 47803 | 4789 | 26.6ms | 45.166ms |
| Hypercorn [POST] (c128) | 43983 | 4404 | 28.926ms | 44.677ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c256) | 379087 | 38024 | 6.7ms | 60.406ms |
| Granian Wsgi [POST] (c128) | 348084 | 34852 | 3.657ms | 33.45ms |
| Gunicorn Gthread [GET] (c64) | 36357 | 3637 | 17.546ms | 22.729ms |
| Gunicorn Gthread [POST] (c64) | 35822 | 3583 | 17.817ms | 20.54ms |
| Gunicorn Gevent [GET] (c64) | 62750 | 6276 | 8.606ms | 4635.689ms |
| Gunicorn Gevent [POST] (c64) | 58511 | 5852 | 8.142ms | 7234.595ms |
| Uwsgi [GET] (c128) | 72025 | 7210 | 17.576ms | 3089.517ms |
| Uwsgi [POST] (c512) | 71103 | 7144 | 63.435ms | 6929.166ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c128) | 365863 | 36640 | 3.482ms | 23.411ms |
| Granian Asgi [POST] (c64) | 195653 | 19570 | 3.263ms | 7.519ms |
| Hypercorn [GET] (c64) | 30733 | 3074 | 20.728ms | 53.841ms |
| Hypercorn [POST] (c64) | 27824 | 2783 | 22.921ms | 54.176ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c64) | 302160 | 30218 | 2.114ms | 4.601ms |
| Uvicorn H11 (c128) | 79436 | 7952 | 16.032ms | 23.631ms |
| Uvicorn Httptools (c64) | 205107 | 20514 | 3.112ms | 6.047ms |
| Hypercorn (c128) | 50181 | 5024 | 25.358ms | 40.241ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 380373 | 38279 | 13.306ms | 122.638ms |
| Granian Rsgi 100ms (c512) | 50025 | 5032 | 100.623ms | 166.445ms |
| Granian Asgi 10ms (c512) | 398875 | 40067 | 12.705ms | 122.106ms |
| Granian Asgi 100ms (c512) | 50138 | 5040 | 100.62ms | 154.915ms |
| Granian Wsgi 10ms (c128) | 111017 | 11114 | 11.476ms | 26.089ms |
| Granian Wsgi 100ms (c512) | 50226 | 5051 | 100.299ms | 154.812ms |
| Uvicorn Httptools 10ms (c512) | 246109 | 24748 | 20.544ms | 115.111ms |
| Uvicorn Httptools 100ms (c512) | 49993 | 5028 | 100.807ms | 183.131ms |
| Hypercorn 10ms (c128) | 49363 | 4943 | 25.764ms | 41.813ms |
| Hypercorn 100ms (c128) | 48959 | 4902 | 25.988ms | 51.059ms |
| Gunicorn Gevent 10ms (c128) | 57255 | 5732 | 22.248ms | 55.119ms |
| Gunicorn Gevent 100ms (c512) | 46614 | 4685 | 108.258ms | 153.266ms |
| Uwsgi 10ms (c128) | 72068 | 7215 | 17.557ms | 2037.695ms |
| Uwsgi 100ms (c256) | 71994 | 7220 | 33.768ms | 6824.297ms |

