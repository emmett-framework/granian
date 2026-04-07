# Granian benchmarks



## Concurrency

Run at: Tue 07 Apr 2026, 12:28    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.12.77 (CPUs: 16)    
Python version: 3.13    
Granian version: 2.7.3    

Same methodology of the main benchmarks applies.

The benchmark consists of an HTTP GET request returning a 1KB plain-text response (the response is a single static byte string).

### Workers

| Interface | Workers | Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- |
| ASGI | 1 | 128 | 1406942 | 140662 | 0.906ms | 73.794ms |
| ASGI | 2 | 256 | 2202674 | 220196 | 1.156ms | 106.876ms |
| ASGI | 4 | 512 | 3585320 | 358413 | 1.414ms | 125.788ms |
| RSGI | 1 | 128 | 1443986 | 144364 | 0.883ms | 48.233ms |
| RSGI | 2 | 256 | 2517139 | 251591 | 1.011ms | 86.788ms |
| RSGI | 4 | 512 | 3856274 | 385396 | 1.316ms | 124.762ms |
| WSGI | 1 | 128 | 1430459 | 143007 | 0.89ms | 69.824ms |
| WSGI | 2 | 256 | 2765506 | 276443 | 0.92ms | 79.723ms |
| WSGI | 4 | 512 | 3937616 | 393561 | 1.288ms | 130.683ms |

### Runtime threads

| Interface | Mode | Threads | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- |
| ASGI | ST | 1 | 1395381 | 139494 | 1.822ms | 127.924ms |
| ASGI | MT | 1 | 1470466 | 147010 | 1.732ms | 100.205ms |
| ASGI | ST | 2 | 1203626 | 120321 | 2.118ms | 66.144ms |
| ASGI | MT | 2 | 1531633 | 153124 | 1.665ms | 65.326ms |
| ASGI | ST | 4 | 1203907 | 120350 | 2.115ms | 79.896ms |
| ASGI | MT | 4 | 1388864 | 138848 | 1.835ms | 74.381ms |
| RSGI | ST | 1 | 1433147 | 143269 | 1.776ms | 104.718ms |
| RSGI | MT | 1 | 1425789 | 142543 | 1.782ms | 103.414ms |
| RSGI | ST | 2 | 1979099 | 197868 | 1.285ms | 106.213ms |
| RSGI | MT | 2 | 2403839 | 240302 | 1.056ms | 134.542ms |
| RSGI | ST | 4 | 1464789 | 146417 | 1.736ms | 96.982ms |
| RSGI | MT | 4 | 2371010 | 236986 | 1.071ms | 112.773ms |
| WSGI | ST | 1 | 1474542 | 147421 | 1.728ms | 86.058ms |
| WSGI | MT | 1 | 1455500 | 145518 | 1.747ms | 105.464ms |
| WSGI | ST | 2 | 1578043 | 157752 | 1.61ms | 133.435ms |
| WSGI | MT | 2 | 2489700 | 248820 | 1.02ms | 104.148ms |
| WSGI | ST | 4 | 1442754 | 144238 | 1.765ms | 96.891ms |
| WSGI | MT | 4 | 2229561 | 222854 | 1.139ms | 134.133ms |
