# Granian benchmarks



## Python versions

Run at: Thu 30 Jan 2025, 03:32    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Granian version: 1.7.6    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.9 | RSGI bytes (c128) | 1311851 | 131227 | 0.975ms | 17.567ms |
| 3.9 | RSGI str (c64) | 1335286 | 133521 | 0.479ms | 1.707ms |
| 3.9 | RSGI echo (c64) | 1176034 | 117605 | 0.543ms | 1.535ms |
| 3.9 | ASGI bytes (c128) | 1239218 | 123972 | 1.032ms | 14.22ms |
| 3.9 | ASGI str (c64) | 1192748 | 119276 | 0.536ms | 2.37ms |
| 3.9 | ASGI echo (c128) | 808472 | 80909 | 1.58ms | 19.078ms |
| 3.9 | WSGI bytes (c64) | 1342118 | 134207 | 0.476ms | 1.542ms |
| 3.9 | WSGI str (c64) | 1339134 | 133907 | 0.477ms | 1.502ms |
| 3.9 | WSGI echo (c64) | 1321322 | 132130 | 0.484ms | 1.55ms |
| 3.10 | RSGI bytes (c128) | 1323978 | 132531 | 0.964ms | 14.834ms |
| 3.10 | RSGI str (c64) | 1340361 | 134034 | 0.477ms | 1.576ms |
| 3.10 | RSGI echo (c128) | 1203623 | 120427 | 1.061ms | 18.959ms |
| 3.10 | ASGI bytes (c128) | 1231374 | 123213 | 1.038ms | 15.009ms |
| 3.10 | ASGI str (c64) | 1206823 | 120683 | 0.53ms | 1.611ms |
| 3.10 | ASGI echo (c64) | 765316 | 76532 | 0.835ms | 2.143ms |
| 3.10 | WSGI bytes (c256) | 1319475 | 132235 | 1.934ms | 17.009ms |
| 3.10 | WSGI str (c64) | 1327550 | 132746 | 0.482ms | 1.563ms |
| 3.10 | WSGI echo (c128) | 1323622 | 132456 | 0.965ms | 13.517ms |
| 3.11 | RSGI bytes (c256) | 901759 | 90302 | 2.833ms | 13.647ms |
| 3.11 | RSGI str (c256) | 885121 | 88661 | 2.885ms | 13.819ms |
| 3.11 | RSGI echo (c128) | 643517 | 64377 | 1.987ms | 13.493ms |
| 3.11 | ASGI bytes (c128) | 716894 | 71717 | 1.783ms | 17.229ms |
| 3.11 | ASGI str (c128) | 718844 | 71915 | 1.778ms | 9.746ms |
| 3.11 | ASGI echo (c64) | 490595 | 49063 | 1.302ms | 2.382ms |
| 3.11 | WSGI bytes (c512) | 642697 | 64468 | 7.932ms | 44.61ms |
| 3.11 | WSGI str (c256) | 632896 | 63398 | 4.034ms | 14.664ms |
| 3.11 | WSGI echo (c256) | 598930 | 59976 | 4.261ms | 23.042ms |
| 3.12 | RSGI bytes (c256) | 895507 | 89667 | 2.853ms | 11.617ms |
| 3.12 | RSGI str (c256) | 898199 | 89988 | 2.843ms | 14.007ms |
| 3.12 | RSGI echo (c128) | 616345 | 61658 | 2.074ms | 15.792ms |
| 3.12 | ASGI bytes (c128) | 659380 | 65961 | 1.939ms | 13.531ms |
| 3.12 | ASGI str (c256) | 652380 | 65326 | 3.916ms | 12.502ms |
| 3.12 | ASGI echo (c64) | 472725 | 47273 | 1.352ms | 2.462ms |
| 3.12 | WSGI bytes (c64) | 597478 | 59751 | 1.069ms | 1.843ms |
| 3.12 | WSGI str (c128) | 582559 | 58295 | 2.193ms | 11.852ms |
| 3.12 | WSGI echo (c64) | 559954 | 55995 | 1.142ms | 1.966ms |
| 3.13 | RSGI bytes (c256) | 897838 | 89954 | 2.843ms | 13.808ms |
| 3.13 | RSGI str (c256) | 914436 | 91586 | 2.793ms | 17.394ms |
| 3.13 | RSGI echo (c256) | 654864 | 65601 | 3.898ms | 15.93ms |
| 3.13 | ASGI bytes (c128) | 748006 | 74827 | 1.709ms | 16.539ms |
| 3.13 | ASGI str (c512) | 676968 | 67945 | 7.526ms | 34.523ms |
| 3.13 | ASGI echo (c64) | 497521 | 49751 | 1.285ms | 2.17ms |
| 3.13 | WSGI bytes (c64) | 630906 | 63095 | 1.013ms | 1.987ms |
| 3.13 | WSGI str (c64) | 620920 | 62092 | 1.03ms | 2.32ms |
| 3.13 | WSGI echo (c128) | 586590 | 58680 | 2.178ms | 18.345ms |
