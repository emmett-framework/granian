# Granian benchmarks



## Python versions

Run at: Mon 01 Dec 2025, 16:46    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.12.58 (CPUs: 16)    
Granian version: 2.6.0    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.10 | RSGI get 1KB (c128) | 1467536 | 146707 | 0.868ms | 57.537ms |
| 3.10 | RSGI echo 1KB (c128) | 1257144 | 125681 | 1.014ms | 61.425ms |
| 3.10 | RSGI echo 100KB (iter) (c64) | 169777 | 16981 | 3.759ms | 40.27ms |
| 3.10 | ASGI get 1KB (c128) | 1294779 | 129445 | 0.984ms | 55.363ms |
| 3.10 | ASGI echo 1KB (c128) | 880548 | 88040 | 1.449ms | 46.504ms |
| 3.10 | ASGI echo 100KB (iter) (c64) | 196847 | 19686 | 3.242ms | 41.974ms |
| 3.10 | WSGI get 1KB (c64) | 1503623 | 150290 | 0.423ms | 49.54ms |
| 3.10 | WSGI echo 1KB (c64) | 1369586 | 136907 | 0.465ms | 38.576ms |
| 3.10 | WSGI echo 100KB (iter) (c64) | 101077 | 10111 | 6.315ms | 45.374ms |
| 3.11 | RSGI get 1KB (c128) | 1458213 | 145771 | 0.872ms | 103.029ms |
| 3.11 | RSGI echo 1KB (c128) | 1275784 | 127539 | 0.998ms | 74.297ms |
| 3.11 | RSGI echo 100KB (iter) (c64) | 166624 | 16663 | 3.833ms | 29.209ms |
| 3.11 | ASGI get 1KB (c128) | 1375259 | 137494 | 0.927ms | 61.955ms |
| 3.11 | ASGI echo 1KB (c128) | 934553 | 93437 | 1.364ms | 55.98ms |
| 3.11 | ASGI echo 100KB (iter) (c64) | 196397 | 19641 | 3.25ms | 41.786ms |
| 3.11 | WSGI get 1KB (c64) | 1473182 | 147273 | 0.433ms | 26.616ms |
| 3.11 | WSGI echo 1KB (c64) | 1380607 | 138008 | 0.462ms | 28.359ms |
| 3.11 | WSGI echo 100KB (iter) (c64) | 105149 | 10518 | 6.062ms | 71.718ms |
| 3.12 | RSGI get 1KB (c128) | 1459399 | 145891 | 0.873ms | 64.057ms |
| 3.12 | RSGI echo 1KB (c128) | 1278308 | 127795 | 0.997ms | 54.248ms |
| 3.12 | RSGI echo 100KB (iter) (c64) | 178453 | 17847 | 3.577ms | 41.065ms |
| 3.12 | ASGI get 1KB (c128) | 1372929 | 137265 | 0.927ms | 88.186ms |
| 3.12 | ASGI echo 1KB (c128) | 919891 | 91970 | 1.386ms | 57.104ms |
| 3.12 | ASGI echo 100KB (iter) (c64) | 194982 | 19500 | 3.275ms | 28.367ms |
| 3.12 | WSGI get 1KB (c64) | 1492775 | 149223 | 0.427ms | 44.059ms |
| 3.12 | WSGI echo 1KB (c64) | 1366811 | 136635 | 0.466ms | 49.445ms |
| 3.12 | WSGI echo 100KB (iter) (c64) | 103796 | 10383 | 6.154ms | 30.083ms |
| 3.13 | RSGI get 1KB (c128) | 1462854 | 146253 | 0.871ms | 66.615ms |
| 3.13 | RSGI echo 1KB (c128) | 1183206 | 118290 | 1.078ms | 42.013ms |
| 3.13 | RSGI echo 100KB (iter) (c64) | 175782 | 17580 | 3.631ms | 41.018ms |
| 3.13 | ASGI get 1KB (c128) | 1221253 | 122088 | 1.044ms | 61.669ms |
| 3.13 | ASGI echo 1KB (c128) | 782970 | 78283 | 1.629ms | 67.885ms |
| 3.13 | ASGI echo 100KB (iter) (c64) | 194463 | 19448 | 3.283ms | 37.874ms |
| 3.13 | WSGI get 1KB (c64) | 1458416 | 145783 | 0.437ms | 38.773ms |
| 3.13 | WSGI echo 1KB (c64) | 1334167 | 133370 | 0.478ms | 23.01ms |
| 3.13 | WSGI echo 100KB (iter) (c64) | 93275 | 9332 | 6.848ms | 32.232ms |
| 3.14 | RSGI get 1KB (c128) | 1463689 | 146319 | 0.872ms | 43.673ms |
| 3.14 | RSGI echo 1KB (c128) | 1281713 | 128132 | 0.994ms | 59.593ms |
| 3.14 | RSGI echo 100KB (iter) (c64) | 171280 | 17131 | 3.729ms | 27.763ms |
| 3.14 | ASGI get 1KB (c128) | 1409591 | 140938 | 0.904ms | 64.846ms |
| 3.14 | ASGI echo 1KB (c128) | 977584 | 97737 | 1.304ms | 72.092ms |
| 3.14 | ASGI echo 100KB (iter) (c64) | 195870 | 19589 | 3.258ms | 46.308ms |
| 3.14 | WSGI get 1KB (c64) | 1481403 | 148095 | 0.43ms | 35.917ms |
| 3.14 | WSGI echo 1KB (c64) | 1343787 | 134321 | 0.474ms | 60.887ms |
| 3.14 | WSGI echo 100KB (iter) (c64) | 102962 | 10301 | 6.199ms | 44.088ms |
