# Granian benchmarks



## Python versions

Run at: Mon 07 Apr 2025, 12:27    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Granian version: 2.2.2    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.9 | RSGI get 1KB (c128) | 1309088 | 131076 | 0.975ms | 17.192ms |
| 3.9 | RSGI echo 1KB (c256) | 1183290 | 118650 | 2.149ms | 46.158ms |
| 3.9 | RSGI echo 100KB (iter) (c64) | 242664 | 24268 | 2.63ms | 11.341ms |
| 3.9 | ASGI get 1KB (c256) | 1281436 | 128350 | 1.993ms | 20.517ms |
| 3.9 | ASGI echo 1KB (c64) | 871957 | 87202 | 0.733ms | 2.511ms |
| 3.9 | ASGI echo 100KB (iter) (c64) | 284194 | 28416 | 2.249ms | 9.79ms |
| 3.9 | WSGI get 1KB (c64) | 1310086 | 131011 | 0.488ms | 1.787ms |
| 3.9 | WSGI echo 1KB (c64) | 1250592 | 125079 | 0.51ms | 2.364ms |
| 3.9 | WSGI echo 100KB (iter) (c64) | 118740 | 11873 | 5.38ms | 19.686ms |
| 3.10 | RSGI get 1KB (c128) | 1296341 | 129708 | 0.986ms | 10.853ms |
| 3.10 | RSGI echo 1KB (c256) | 1206429 | 120870 | 2.114ms | 49.139ms |
| 3.10 | RSGI echo 100KB (iter) (c64) | 212002 | 21200 | 3.013ms | 10.411ms |
| 3.10 | ASGI get 1KB (c256) | 1287355 | 128967 | 1.983ms | 27.977ms |
| 3.10 | ASGI echo 1KB (c128) | 901423 | 90215 | 1.415ms | 21.668ms |
| 3.10 | ASGI echo 100KB (iter) (c64) | 217997 | 21802 | 2.928ms | 9.892ms |
| 3.10 | WSGI get 1KB (c256) | 1294100 | 129901 | 1.967ms | 30.996ms |
| 3.10 | WSGI echo 1KB (c64) | 1267040 | 126706 | 0.504ms | 2.287ms |
| 3.10 | WSGI echo 100KB (iter) (c64) | 118346 | 11834 | 5.398ms | 19.235ms |
| 3.11 | RSGI get 1KB (c512) | 790095 | 79458 | 6.436ms | 74.526ms |
| 3.11 | RSGI echo 1KB (c128) | 578324 | 57916 | 2.203ms | 20.423ms |
| 3.11 | RSGI echo 100KB (iter) (c64) | 232638 | 23264 | 2.747ms | 9.956ms |
| 3.11 | ASGI get 1KB (c128) | 630602 | 63099 | 2.023ms | 21.42ms |
| 3.11 | ASGI echo 1KB (c128) | 462552 | 46296 | 2.756ms | 26.087ms |
| 3.11 | ASGI echo 100KB (iter) (c64) | 236152 | 23616 | 2.703ms | 11.58ms |
| 3.11 | WSGI get 1KB (c64) | 702257 | 70236 | 0.909ms | 1.832ms |
| 3.11 | WSGI echo 1KB (c256) | 602204 | 60368 | 4.235ms | 26.952ms |
| 3.11 | WSGI echo 100KB (iter) (c64) | 87141 | 8715 | 7.321ms | 25.406ms |
| 3.12 | RSGI get 1KB (c256) | 792385 | 79476 | 3.216ms | 21.257ms |
| 3.12 | RSGI echo 1KB (c128) | 567498 | 56798 | 2.251ms | 12.582ms |
| 3.12 | RSGI echo 100KB (iter) (c64) | 214547 | 21457 | 2.976ms | 9.891ms |
| 3.12 | ASGI get 1KB (c128) | 618635 | 61910 | 2.065ms | 30.167ms |
| 3.12 | ASGI echo 1KB (c64) | 439767 | 43978 | 1.453ms | 4.121ms |
| 3.12 | ASGI echo 100KB (iter) (c64) | 224347 | 22434 | 2.849ms | 10.07ms |
| 3.12 | WSGI get 1KB (c256) | 639132 | 64156 | 3.984ms | 27.629ms |
| 3.12 | WSGI echo 1KB (c128) | 621498 | 62212 | 2.055ms | 14.152ms |
| 3.12 | WSGI echo 100KB (iter) (c64) | 86523 | 8652 | 7.383ms | 26.81ms |
| 3.13 | RSGI get 1KB (c256) | 869566 | 87154 | 2.934ms | 25.126ms |
| 3.13 | RSGI echo 1KB (c128) | 594639 | 59498 | 2.145ms | 22.636ms |
| 3.13 | RSGI echo 100KB (iter) (c64) | 229289 | 22929 | 2.786ms | 10.777ms |
| 3.13 | ASGI get 1KB (c256) | 640260 | 64160 | 3.986ms | 25.69ms |
| 3.13 | ASGI echo 1KB (c128) | 436580 | 43712 | 2.919ms | 20.031ms |
| 3.13 | ASGI echo 100KB (iter) (c64) | 232578 | 23257 | 2.748ms | 9.376ms |
| 3.13 | WSGI get 1KB (c64) | 641504 | 64152 | 0.996ms | 1.76ms |
| 3.13 | WSGI echo 1KB (c64) | 615455 | 61549 | 1.038ms | 1.782ms |
| 3.13 | WSGI echo 100KB (iter) (c64) | 86468 | 8647 | 7.385ms | 27.66ms |
