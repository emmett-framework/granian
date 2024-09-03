# Granian benchmarks



## VS 3rd party comparison

Run at: Tue 03 Sep 2024, 21:44    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.6.0    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c128) | 453458 | 45416 | 2.806ms | 31.751ms |
| Granian Asgi [POST] (c64) | 219795 | 21983 | 2.904ms | 6.354ms |
| Uvicorn H11 [GET] (c64) | 80046 | 8006 | 7.972ms | 18.857ms |
| Uvicorn H11 [POST] (c64) | 71376 | 7139 | 8.942ms | 24.293ms |
| Uvicorn Httptools [GET] (c128) | 371460 | 37210 | 3.424ms | 32.85ms |
| Uvicorn Httptools [POST] (c128) | 342512 | 34301 | 3.718ms | 24.748ms |
| Hypercorn [GET] (c128) | 49725 | 4978 | 25.595ms | 31.829ms |
| Hypercorn [POST] (c128) | 45388 | 4547 | 28.022ms | 31.55ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c256) | 380226 | 38107 | 6.691ms | 57.374ms |
| Granian Wsgi [POST] (c128) | 332496 | 33295 | 3.829ms | 33.119ms |
| Gunicorn Gthread [GET] (c64) | 36866 | 3687 | 17.304ms | 19.372ms |
| Gunicorn Gthread [POST] (c64) | 35502 | 3551 | 17.962ms | 20.207ms |
| Gunicorn Gevent [GET] (c64) | 63589 | 6361 | 6.593ms | 7589.315ms |
| Gunicorn Gevent [POST] (c256) | 60097 | 6033 | 7.233ms | 9884.01ms |
| Uwsgi [GET] (c256) | 72701 | 7286 | 33.627ms | 4735.69ms |
| Uwsgi [POST] (c256) | 72121 | 7227 | 34.111ms | 3470.737ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c64) | 373977 | 37408 | 1.706ms | 6.236ms |
| Granian Asgi [POST] (c256) | 197345 | 19804 | 12.862ms | 92.805ms |
| Hypercorn [GET] (c64) | 31630 | 3164 | 20.144ms | 48.95ms |
| Hypercorn [POST] (c64) | 28163 | 2817 | 22.641ms | 69.083ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c64) | 301264 | 30123 | 2.12ms | 5.324ms |
| Uvicorn H11 (c128) | 78809 | 7893 | 16.152ms | 27.46ms |
| Uvicorn Httptools (c128) | 207081 | 20727 | 6.152ms | 27.753ms |
| Hypercorn (c128) | 49403 | 4945 | 25.77ms | 32.498ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 400686 | 40280 | 12.66ms | 105.734ms |
| Granian Rsgi 100ms (c512) | 50049 | 5033 | 100.68ms | 157.381ms |
| Granian Asgi 10ms (c512) | 400154 | 40203 | 12.673ms | 113.876ms |
| Granian Asgi 100ms (c512) | 50070 | 5037 | 100.641ms | 171.637ms |
| Granian Wsgi 10ms (c128) | 112522 | 11265 | 11.315ms | 31.271ms |
| Granian Wsgi 100ms (c512) | 50253 | 5054 | 100.295ms | 150.603ms |
| Uvicorn Httptools 10ms (c512) | 246558 | 24794 | 20.545ms | 104.779ms |
| Uvicorn Httptools 100ms (c512) | 50019 | 5028 | 100.811ms | 174.095ms |
| Hypercorn 10ms (c128) | 49761 | 4982 | 25.581ms | 42.662ms |
| Hypercorn 100ms (c128) | 49443 | 4949 | 25.719ms | 43.894ms |
| Gunicorn Gevent 10ms (c64) | 57483 | 5750 | 11.088ms | 25.63ms |
| Gunicorn Gevent 100ms (c512) | 48377 | 4862 | 104.317ms | 171.487ms |
| Uwsgi 10ms (c256) | 72900 | 7311 | 34.199ms | 6609.152ms |
| Uwsgi 100ms (c512) | 72643 | 7304 | 61.63ms | 7512.077ms |

