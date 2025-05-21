# Granian benchmarks



## Python versions

Run at: Wed 21 May 2025, 01:11    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Granian version: 2.3.1    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.9 | RSGI get 1KB (c64) | 1318180 | 131827 | 0.485ms | 1.861ms |
| 3.9 | RSGI echo 1KB (c256) | 1233475 | 123516 | 2.066ms | 43.443ms |
| 3.9 | RSGI echo 100KB (iter) (c128) | 186170 | 18639 | 6.84ms | 40.361ms |
| 3.9 | ASGI get 1KB (c128) | 1291529 | 129254 | 0.989ms | 24.662ms |
| 3.9 | ASGI echo 1KB (c128) | 945443 | 94581 | 1.351ms | 9.619ms |
| 3.9 | ASGI echo 100KB (iter) (c64) | 305748 | 30572 | 2.09ms | 9.232ms |
| 3.9 | WSGI get 1KB (c64) | 1289420 | 128941 | 0.496ms | 1.66ms |
| 3.9 | WSGI echo 1KB (c256) | 1247884 | 125192 | 2.043ms | 28.183ms |
| 3.9 | WSGI echo 100KB (iter) (c64) | 88688 | 8869 | 7.2ms | 22.603ms |
| 3.10 | RSGI get 1KB (c64) | 1311579 | 131173 | 0.487ms | 2.285ms |
| 3.10 | RSGI echo 1KB (c64) | 1259827 | 125991 | 0.507ms | 1.848ms |
| 3.10 | RSGI echo 100KB (iter) (c128) | 185739 | 18589 | 6.859ms | 42.101ms |
| 3.10 | ASGI get 1KB (c128) | 1288863 | 129015 | 0.991ms | 19.146ms |
| 3.10 | ASGI echo 1KB (c128) | 957195 | 95814 | 1.331ms | 26.642ms |
| 3.10 | ASGI echo 100KB (iter) (c64) | 196080 | 19610 | 3.255ms | 9.815ms |
| 3.10 | WSGI get 1KB (c64) | 1331981 | 133198 | 0.48ms | 1.684ms |
| 3.10 | WSGI echo 1KB (c256) | 1255118 | 125762 | 2.028ms | 44.995ms |
| 3.10 | WSGI echo 100KB (iter) (c64) | 91034 | 9103 | 7.019ms | 21.309ms |
| 3.11 | RSGI get 1KB (c256) | 1273730 | 127769 | 2.001ms | 28.588ms |
| 3.11 | RSGI echo 1KB (c64) | 1237142 | 123718 | 0.517ms | 1.669ms |
| 3.11 | RSGI echo 100KB (iter) (c128) | 191242 | 19134 | 6.668ms | 35.035ms |
| 3.11 | ASGI get 1KB (c128) | 1295642 | 129679 | 0.985ms | 20.977ms |
| 3.11 | ASGI echo 1KB (c128) | 1005641 | 100628 | 1.269ms | 17.658ms |
| 3.11 | ASGI echo 100KB (iter) (c64) | 192239 | 19224 | 3.325ms | 9.743ms |
| 3.11 | WSGI get 1KB (c128) | 1285015 | 128630 | 0.994ms | 24.101ms |
| 3.11 | WSGI echo 1KB (c64) | 1256898 | 125691 | 0.508ms | 1.594ms |
| 3.11 | WSGI echo 100KB (iter) (c64) | 92290 | 9229 | 6.924ms | 24.891ms |
| 3.12 | RSGI get 1KB (c128) | 1272660 | 127371 | 1.004ms | 28.947ms |
| 3.12 | RSGI echo 1KB (c256) | 1226671 | 122797 | 2.079ms | 34.14ms |
| 3.12 | RSGI echo 100KB (iter) (c128) | 194727 | 19478 | 6.557ms | 28.2ms |
| 3.12 | ASGI get 1KB (c128) | 1287406 | 128862 | 0.992ms | 21.728ms |
| 3.12 | ASGI echo 1KB (c128) | 932483 | 93364 | 1.366ms | 24.462ms |
| 3.12 | ASGI echo 100KB (iter) (c64) | 198252 | 19825 | 3.223ms | 10.058ms |
| 3.12 | WSGI get 1KB (c64) | 1310953 | 131113 | 0.487ms | 1.995ms |
| 3.12 | WSGI echo 1KB (c64) | 1247872 | 124800 | 0.512ms | 3.627ms |
| 3.12 | WSGI echo 100KB (iter) (c64) | 89620 | 8963 | 7.119ms | 24.27ms |
| 3.13 | RSGI get 1KB (c128) | 1292391 | 129419 | 0.987ms | 27.664ms |
| 3.13 | RSGI echo 1KB (c64) | 1234224 | 123429 | 0.517ms | 1.576ms |
| 3.13 | RSGI echo 100KB (iter) (c64) | 284835 | 28481 | 2.243ms | 10.913ms |
| 3.13 | ASGI get 1KB (c128) | 1268821 | 127067 | 1.005ms | 26.108ms |
| 3.13 | ASGI echo 1KB (c128) | 949255 | 95001 | 1.343ms | 22.869ms |
| 3.13 | ASGI echo 100KB (iter) (c64) | 205253 | 20528 | 3.109ms | 10.182ms |
| 3.13 | WSGI get 1KB (c128) | 1293463 | 129521 | 0.986ms | 26.921ms |
| 3.13 | WSGI echo 1KB (c256) | 1228702 | 123243 | 2.069ms | 47.445ms |
| 3.13 | WSGI echo 100KB (iter) (c64) | 89881 | 8988 | 7.105ms | 22.957ms |
