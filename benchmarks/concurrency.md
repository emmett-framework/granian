# Granian benchmarks



## Concurrency

Run at: Wed 05 Aug 2026, 17:48    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.18.41 (CPUs: 16)    
Python version: 3.13    
Granian version: 2.8.1    

Same methodology of the main benchmarks applies.

The benchmark consists of an HTTP GET request returning a 1KB plain-text response (the response is a single static byte string).

### Workers

| Interface | Workers | Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- |
| ASGI | 1 | 128 | 1427457 | 142701 | 0.892ms | 53.412ms |
| ASGI | 2 | 256 | 2267034 | 226623 | 1.124ms | 80.665ms |
| ASGI | 4 | 512 | 3594887 | 359290 | 1.413ms | 108.901ms |
| RSGI | 1 | 128 | 1460231 | 146000 | 0.873ms | 52.356ms |
| RSGI | 2 | 256 | 2572549 | 257192 | 0.99ms | 91.673ms |
| RSGI | 4 | 512 | 3879891 | 387862 | 1.311ms | 93.01ms |
| WSGI | 1 | 128 | 1435367 | 143520 | 0.888ms | 45.527ms |
| WSGI | 2 | 256 | 2771339 | 276961 | 0.918ms | 96.792ms |
| WSGI | 4 | 512 | 3976050 | 397374 | 1.279ms | 100.516ms |

### Runtime threads

| Interface | Mode | Threads | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- |
| ASGI | ST | 1 | 1320968 | 132067 | 1.929ms | 88.801ms |
| ASGI | MT | 1 | 1416808 | 141657 | 1.8ms | 77.48ms |
| ASGI | ST | 2 | 1132347 | 113204 | 2.252ms | 68.325ms |
| ASGI | MT | 2 | 1427960 | 142759 | 1.784ms | 78.175ms |
| ASGI | ST | 4 | 1082549 | 108238 | 2.35ms | 98.073ms |
| ASGI | MT | 4 | 1291990 | 129162 | 1.969ms | 104.095ms |
| RSGI | ST | 1 | 1424342 | 142402 | 1.789ms | 89.363ms |
| RSGI | MT | 1 | 1437872 | 143745 | 1.768ms | 115.531ms |
| RSGI | ST | 2 | 1846124 | 184555 | 1.38ms | 81.568ms |
| RSGI | MT | 2 | 2400768 | 239970 | 1.06ms | 102.309ms |
| RSGI | ST | 4 | 1430437 | 143010 | 1.777ms | 114.816ms |
| RSGI | MT | 4 | 2391188 | 239000 | 1.064ms | 96.144ms |
| WSGI | ST | 1 | 1467930 | 146744 | 1.735ms | 87.341ms |
| WSGI | MT | 1 | 1412633 | 141219 | 1.802ms | 103.84ms |
| WSGI | ST | 2 | 1632003 | 163167 | 1.562ms | 69.794ms |
| WSGI | MT | 2 | 2453041 | 245218 | 1.039ms | 71.897ms |
| WSGI | ST | 4 | 1461584 | 146126 | 1.738ms | 128.644ms |
| WSGI | MT | 4 | 2480051 | 247909 | 1.027ms | 62.702ms |
