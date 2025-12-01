# Granian benchmarks



## Concurrency

Run at: Mon 01 Dec 2025, 17:15    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.12.58 (CPUs: 16)    
Python version: 3.13    
Granian version: 2.6.0    

Same methodology of the main benchmarks applies.

The benchmark consists of an HTTP GET request returning a 1KB plain-text response (the response is a single static byte string).

### Workers

| Interface | Workers | Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- |
| ASGI | 1 | 128 | 1196547 | 119623 | 1.066ms | 56.137ms |
| ASGI | 2 | 256 | 1875440 | 187468 | 1.358ms | 95.707ms |
| ASGI | 4 | 512 | 2883397 | 288213 | 1.764ms | 104.069ms |
| RSGI | 1 | 128 | 1476829 | 147647 | 0.863ms | 50.726ms |
| RSGI | 2 | 256 | 2540977 | 253951 | 1.001ms | 84.226ms |
| RSGI | 4 | 512 | 3777805 | 377549 | 1.345ms | 120.704ms |
| WSGI | 1 | 128 | 1427206 | 142682 | 0.893ms | 53.685ms |
| WSGI | 2 | 256 | 2390255 | 238898 | 1.065ms | 92.968ms |
| WSGI | 4 | 512 | 3737615 | 373516 | 1.359ms | 119.637ms |

### Runtime threads

| Interface | Mode | Threads | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- |
| ASGI | ST | 1 | 1229064 | 122878 | 2.069ms | 104.99ms |
| ASGI | MT | 1 | 1279139 | 127876 | 1.988ms | 107.788ms |
| ASGI | ST | 2 | 951067 | 95096 | 2.677ms | 97.518ms |
| ASGI | MT | 2 | 1292021 | 129176 | 1.969ms | 111.181ms |
| ASGI | ST | 4 | 980020 | 97970 | 2.598ms | 102.125ms |
| ASGI | MT | 4 | 1103859 | 110349 | 2.306ms | 92.982ms |
| RSGI | ST | 1 | 1459212 | 145895 | 1.744ms | 83.974ms |
| RSGI | MT | 1 | 1456940 | 145661 | 1.75ms | 79.875ms |
| RSGI | ST | 2 | 1563374 | 156313 | 1.629ms | 78.179ms |
| RSGI | MT | 2 | 2069827 | 206894 | 1.227ms | 129.828ms |
| RSGI | ST | 4 | 1235054 | 123486 | 2.063ms | 85.976ms |
| RSGI | MT | 4 | 1808837 | 180844 | 1.403ms | 120.596ms |
| WSGI | ST | 1 | 1424431 | 142411 | 1.788ms | 85.669ms |
| WSGI | MT | 1 | 1475486 | 147519 | 1.728ms | 66.107ms |
| WSGI | ST | 2 | 1262265 | 126202 | 2.001ms | 296.747ms |
| WSGI | MT | 2 | 1812351 | 181178 | 1.405ms | 70.036ms |
| WSGI | ST | 4 | 1208870 | 120857 | 2.094ms | 330.131ms |
| WSGI | MT | 4 | 1648401 | 164759 | 1.541ms | 114.446ms |
