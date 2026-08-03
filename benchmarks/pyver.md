# Granian benchmarks



## Python versions

Run at: Mon 03 Aug 2026, 13:13    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.18.41 (CPUs: 16)    
Granian version: 2.8.0    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.10 | RSGI get 1KB (c128) | 1423176 | 142282 | 0.895ms | 54.738ms |
| 3.10 | RSGI echo 1KB (c128) | 1250707 | 125040 | 1.02ms | 58.04ms |
| 3.10 | RSGI echo 100KB (iter) (c64) | 165874 | 16591 | 3.849ms | 32.522ms |
| 3.10 | ASGI get 1KB (c128) | 1256037 | 125583 | 1.015ms | 42.908ms |
| 3.10 | ASGI echo 1KB (c128) | 843722 | 84365 | 1.512ms | 44.005ms |
| 3.10 | ASGI echo 100KB (iter) (c64) | 190144 | 19016 | 3.358ms | 36.053ms |
| 3.10 | WSGI get 1KB (c64) | 1444814 | 144427 | 0.441ms | 22.574ms |
| 3.10 | WSGI echo 1KB (c64) | 1378849 | 137838 | 0.462ms | 36.637ms |
| 3.10 | WSGI echo 100KB (iter) (c64) | 97508 | 9756 | 6.547ms | 41.129ms |
| 3.11 | RSGI get 1KB (c128) | 1440794 | 144051 | 0.884ms | 72.249ms |
| 3.11 | RSGI echo 1KB (c128) | 1268594 | 126831 | 1.005ms | 49.09ms |
| 3.11 | RSGI echo 100KB (iter) (c64) | 172770 | 17280 | 3.698ms | 24.174ms |
| 3.11 | ASGI get 1KB (c128) | 1324715 | 132443 | 0.963ms | 41.084ms |
| 3.11 | ASGI echo 1KB (c128) | 889030 | 88890 | 1.436ms | 36.3ms |
| 3.11 | ASGI echo 100KB (iter) (c64) | 188621 | 18864 | 3.383ms | 43.642ms |
| 3.11 | WSGI get 1KB (c64) | 1441575 | 144107 | 0.442ms | 34.028ms |
| 3.11 | WSGI echo 1KB (c64) | 1338565 | 133813 | 0.476ms | 26.02ms |
| 3.11 | WSGI echo 100KB (iter) (c64) | 100153 | 10020 | 6.377ms | 35.716ms |
| 3.12 | RSGI get 1KB (c128) | 1457218 | 145685 | 0.874ms | 52.929ms |
| 3.12 | RSGI echo 1KB (c128) | 1278485 | 127806 | 0.997ms | 47.665ms |
| 3.12 | RSGI echo 100KB (iter) (c64) | 175086 | 17511 | 3.646ms | 40.673ms |
| 3.12 | ASGI get 1KB (c128) | 1342246 | 134184 | 0.949ms | 70.919ms |
| 3.12 | ASGI echo 1KB (c128) | 883304 | 88323 | 1.443ms | 70.457ms |
| 3.12 | ASGI echo 100KB (iter) (c64) | 190451 | 19047 | 3.35ms | 44.433ms |
| 3.12 | WSGI get 1KB (c64) | 1401453 | 140099 | 0.455ms | 33.79ms |
| 3.12 | WSGI echo 1KB (c64) | 1373215 | 137271 | 0.464ms | 24.401ms |
| 3.12 | WSGI echo 100KB (iter) (c64) | 101033 | 10108 | 6.32ms | 36.662ms |
| 3.13 | RSGI get 1KB (c128) | 1442629 | 144239 | 0.884ms | 43.269ms |
| 3.13 | RSGI echo 1KB (c128) | 1275687 | 127534 | 1.0ms | 36.189ms |
| 3.13 | RSGI echo 100KB (iter) (c64) | 166441 | 16645 | 3.835ms | 38.996ms |
| 3.13 | ASGI get 1KB (c128) | 1335677 | 133519 | 0.954ms | 54.14ms |
| 3.13 | ASGI echo 1KB (c128) | 901620 | 90148 | 1.414ms | 71.392ms |
| 3.13 | ASGI echo 100KB (iter) (c64) | 184943 | 18496 | 3.452ms | 34.549ms |
| 3.13 | WSGI get 1KB (c64) | 1429078 | 142851 | 0.446ms | 19.985ms |
| 3.13 | WSGI echo 1KB (c64) | 1373567 | 137306 | 0.464ms | 31.175ms |
| 3.13 | WSGI echo 100KB (iter) (c64) | 100727 | 10077 | 6.339ms | 38.985ms |
| 3.14 | RSGI get 1KB (c128) | 1468149 | 146774 | 0.868ms | 50.737ms |
| 3.14 | RSGI echo 1KB (c128) | 1277804 | 127742 | 0.998ms | 43.152ms |
| 3.14 | RSGI echo 100KB (iter) (c64) | 175715 | 17574 | 3.634ms | 28.608ms |
| 3.14 | ASGI get 1KB (c128) | 1397437 | 139718 | 0.913ms | 43.069ms |
| 3.14 | ASGI echo 1KB (c128) | 970362 | 97020 | 1.315ms | 37.609ms |
| 3.14 | ASGI echo 100KB (iter) (c64) | 195901 | 19592 | 3.258ms | 41.364ms |
| 3.14 | WSGI get 1KB (c64) | 1456279 | 145576 | 0.437ms | 34.439ms |
| 3.14 | WSGI echo 1KB (c64) | 1373114 | 137267 | 0.464ms | 32.22ms |
| 3.14 | WSGI echo 100KB (iter) (c64) | 100983 | 10103 | 6.324ms | 37.137ms |
