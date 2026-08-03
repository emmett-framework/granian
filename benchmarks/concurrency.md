# Granian benchmarks



## Concurrency

Run at: Mon 03 Aug 2026, 13:41    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.18.41 (CPUs: 16)    
Python version: 3.13    
Granian version: 2.8.0    

Same methodology of the main benchmarks applies.

The benchmark consists of an HTTP GET request returning a 1KB plain-text response (the response is a single static byte string).

### Workers

| Interface | Workers | Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- |
| ASGI | 1 | 128 | 1319411 | 131913 | 0.967ms | 45.407ms |
| ASGI | 2 | 256 | 2175113 | 217448 | 1.172ms | 74.116ms |
| ASGI | 4 | 512 | 3564011 | 356271 | 1.426ms | 99.849ms |
| RSGI | 1 | 128 | 1449693 | 144942 | 0.879ms | 41.042ms |
| RSGI | 2 | 256 | 2584322 | 258330 | 0.986ms | 67.856ms |
| RSGI | 4 | 512 | 3861263 | 385983 | 1.315ms | 107.334ms |
| WSGI | 1 | 128 | 1469809 | 146953 | 0.868ms | 36.424ms |
| WSGI | 2 | 256 | 2714395 | 271343 | 0.939ms | 71.149ms |
| WSGI | 4 | 512 | 3929950 | 392828 | 1.291ms | 121.812ms |

### Runtime threads

| Interface | Mode | Threads | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- |
| ASGI | ST | 1 | 1351261 | 135096 | 1.886ms | 71.585ms |
| ASGI | MT | 1 | 1408538 | 140818 | 1.809ms | 85.866ms |
| ASGI | ST | 2 | 1145612 | 114542 | 2.225ms | 69.063ms |
| ASGI | MT | 2 | 1449934 | 144965 | 1.756ms | 98.348ms |
| ASGI | ST | 4 | 1080964 | 108077 | 2.355ms | 90.897ms |
| ASGI | MT | 4 | 1336276 | 133579 | 1.905ms | 95.431ms |
| RSGI | ST | 1 | 1437429 | 143711 | 1.773ms | 86.036ms |
| RSGI | MT | 1 | 1428261 | 142809 | 1.783ms | 94.274ms |
| RSGI | ST | 2 | 1911568 | 191086 | 1.33ms | 109.83ms |
| RSGI | MT | 2 | 2400416 | 239947 | 1.06ms | 100.88ms |
| RSGI | ST | 4 | 1432782 | 143247 | 1.776ms | 105.56ms |
| RSGI | MT | 4 | 2413049 | 241220 | 1.055ms | 85.182ms |
| WSGI | ST | 1 | 1427549 | 142722 | 1.782ms | 110.991ms |
| WSGI | MT | 1 | 1426695 | 142632 | 1.784ms | 108.254ms |
| WSGI | ST | 2 | 1645860 | 164560 | 1.548ms | 85.402ms |
| WSGI | MT | 2 | 2403642 | 240272 | 1.059ms | 96.24ms |
| WSGI | ST | 4 | 1491180 | 149070 | 1.707ms | 102.626ms |
| WSGI | MT | 4 | 2496248 | 249550 | 1.021ms | 62.297ms |
