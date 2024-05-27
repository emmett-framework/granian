# Granian benchmarks



## VS 3rd party comparison

Run at: Mon 27 May 2024, 06:53    
Environment: GHA Linux x86_64 (CPUs: 4)    
Python version: 3.11    
Granian version: 1.4.0    

### ASGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c512) | 591533 | 39537 | 12.905ms | 146.272ms |
| Granian Asgi [POST] (c128) | 365564 | 24389 | 5.234ms | 33.076ms |
| Uvicorn H11 [GET] (c64) | 119609 | 7975 | 8.01ms | 17.699ms |
| Uvicorn H11 [POST] (c128) | 107295 | 7158 | 17.839ms | 26.697ms |
| Uvicorn Httptools [GET] (c128) | 554481 | 36989 | 3.454ms | 21.168ms |
| Uvicorn Httptools [POST] (c128) | 503214 | 33572 | 3.803ms | 26.156ms |
| Hypercorn [GET] (c128) | 74857 | 4995 | 25.552ms | 29.311ms |
| Hypercorn [POST] (c128) | 69404 | 4630 | 27.561ms | 53.538ms |


### WSGI

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Wsgi [GET] (c256) | 613981 | 41011 | 6.223ms | 67.584ms |
| Granian Wsgi [POST] (c128) | 536063 | 35770 | 3.571ms | 18.16ms |
| Gunicorn Gthread [GET] (c64) | 55314 | 3688 | 17.311ms | 20.787ms |
| Gunicorn Gthread [POST] (c64) | 53340 | 3557 | 17.952ms | 19.994ms |
| Gunicorn Gevent [GET] (c128) | 93675 | 6250 | 13.768ms | 9868.424ms |
| Gunicorn Gevent [POST] (c512) | 87480 | 5852 | 18.193ms | 14848.519ms |
| Uwsgi [GET] (c64) | 108473 | 7232 | 8.835ms | 25.839ms |
| Uwsgi [POST] (c256) | 107609 | 7187 | 34.991ms | 3368.516ms |


### HTTP/2

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Asgi [GET] (c128) | 547711 | 36541 | 3.496ms | 28.719ms |
| Granian Asgi [POST] (c64) | 322277 | 21488 | 2.974ms | 6.605ms |
| Hypercorn [GET] (c256) | 22713 | 1517 | 165.289ms | 593.967ms |
| Hypercorn [POST] (c64) | 42739 | 2850 | 22.408ms | 66.85ms |


### ASGI file responses

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian (pathsend) (c64) | 447477 | 29834 | 2.142ms | 5.706ms |
| Uvicorn H11 (c64) | 118854 | 7924 | 8.063ms | 23.481ms |
| Uvicorn Httptools (c128) | 310254 | 20696 | 6.172ms | 21.613ms |
| Hypercorn (c128) | 75197 | 5017 | 25.44ms | 39.018ms |


### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian Rsgi 10ms (c512) | 540476 | 36164 | 14.104ms | 129.672ms |
| Granian Rsgi 100ms (c512) | 75412 | 5049 | 100.68ms | 166.538ms |
| Granian Asgi 10ms (c512) | 593469 | 39734 | 12.847ms | 131.563ms |
| Granian Asgi 100ms (c512) | 75610 | 5060 | 100.593ms | 151.221ms |
| Granian Wsgi 10ms (c128) | 179570 | 11979 | 10.659ms | 26.765ms |
| Granian Wsgi 100ms (c512) | 75806 | 5073 | 100.304ms | 169.675ms |
| Uvicorn Httptools 10ms (c512) | 365947 | 24473 | 20.831ms | 112.223ms |
| Uvicorn Httptools 100ms (c512) | 75382 | 5042 | 100.861ms | 187.898ms |
| Hypercorn 10ms (c128) | 75109 | 5012 | 25.458ms | 40.094ms |
| Hypercorn 100ms (c128) | 74670 | 4982 | 25.604ms | 55.009ms |
| Gunicorn Gevent 10ms (c128) | 86951 | 5802 | 22.004ms | 55.52ms |
| Gunicorn Gevent 100ms (c512) | 73113 | 4894 | 103.898ms | 199.96ms |
| Uwsgi 10ms (c128) | 106857 | 7132 | 17.798ms | 3298.008ms |
| Uwsgi 100ms (c512) | 108935 | 7288 | 62.962ms | 10635.392ms |

