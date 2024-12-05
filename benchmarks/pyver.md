# Granian benchmarks



## Python versions

Run at: Thu 05 Dec 2024, 19:19    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Granian version: 1.7.0    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.9 | RSGI bytes (c64) | 1321556 | 132150 | 0.484ms | 1.496ms |
| 3.9 | RSGI str (c64) | 1314383 | 131441 | 0.486ms | 1.637ms |
| 3.9 | RSGI echo (c512) | 854276 | 85770 | 5.962ms | 43.878ms |
| 3.9 | ASGI bytes (c64) | 1311686 | 131176 | 0.487ms | 2.455ms |
| 3.9 | ASGI str (c128) | 1315554 | 131625 | 0.971ms | 10.937ms |
| 3.9 | ASGI echo (c128) | 899181 | 89981 | 1.42ms | 11.494ms |
| 3.9 | WSGI bytes (c64) | 1324932 | 132489 | 0.483ms | 1.653ms |
| 3.9 | WSGI str (c64) | 1305102 | 130505 | 0.49ms | 1.556ms |
| 3.9 | WSGI echo (c64) | 1311938 | 131187 | 0.487ms | 2.577ms |
| 3.10 | RSGI bytes (c64) | 1327959 | 132793 | 0.481ms | 2.307ms |
| 3.10 | RSGI str (c128) | 1318196 | 131891 | 0.969ms | 18.843ms |
| 3.10 | RSGI echo (c512) | 875041 | 87820 | 5.824ms | 48.898ms |
| 3.10 | ASGI bytes (c64) | 1320131 | 132006 | 0.484ms | 1.609ms |
| 3.10 | ASGI str (c64) | 1303674 | 130363 | 0.49ms | 1.498ms |
| 3.10 | ASGI echo (c64) | 890942 | 89094 | 0.718ms | 1.57ms |
| 3.10 | WSGI bytes (c64) | 1325236 | 132521 | 0.482ms | 1.565ms |
| 3.10 | WSGI str (c64) | 1337376 | 133736 | 0.478ms | 1.709ms |
| 3.10 | WSGI echo (c64) | 1324390 | 132436 | 0.483ms | 1.527ms |
| 3.11 | RSGI bytes (c256) | 979695 | 98170 | 2.605ms | 16.536ms |
| 3.11 | RSGI str (c256) | 979044 | 98034 | 2.609ms | 14.028ms |
| 3.11 | RSGI echo (c512) | 514804 | 51668 | 9.899ms | 40.282ms |
| 3.11 | ASGI bytes (c256) | 753404 | 75416 | 3.388ms | 30.286ms |
| 3.11 | ASGI str (c128) | 770893 | 77119 | 1.658ms | 15.361ms |
| 3.11 | ASGI echo (c512) | 466150 | 46726 | 10.942ms | 42.779ms |
| 3.11 | WSGI bytes (c64) | 642988 | 64298 | 0.994ms | 1.816ms |
| 3.11 | WSGI str (c128) | 606384 | 60661 | 2.108ms | 11.511ms |
| 3.11 | WSGI echo (c256) | 589528 | 59049 | 4.33ms | 13.673ms |
| 3.12 | RSGI bytes (c256) | 987209 | 98849 | 2.585ms | 27.041ms |
| 3.12 | RSGI str (c256) | 992975 | 99511 | 2.57ms | 15.944ms |
| 3.12 | RSGI echo (c512) | 504614 | 50645 | 10.093ms | 41.187ms |
| 3.12 | ASGI bytes (c256) | 727276 | 72832 | 3.512ms | 17.573ms |
| 3.12 | ASGI str (c256) | 760182 | 76107 | 3.36ms | 16.687ms |
| 3.12 | ASGI echo (c256) | 448423 | 44926 | 5.69ms | 17.904ms |
| 3.12 | WSGI bytes (c128) | 622144 | 62252 | 2.054ms | 7.601ms |
| 3.12 | WSGI str (c256) | 603226 | 60458 | 4.229ms | 13.546ms |
| 3.12 | WSGI echo (c64) | 575707 | 57572 | 1.11ms | 1.793ms |
| 3.13 | RSGI bytes (c512) | 1015261 | 101920 | 5.02ms | 34.988ms |
| 3.13 | RSGI str (c512) | 1010240 | 101369 | 5.048ms | 47.614ms |
| 3.13 | RSGI echo (c512) | 541106 | 54237 | 9.429ms | 36.025ms |
| 3.13 | ASGI bytes (c256) | 766256 | 76704 | 3.331ms | 30.396ms |
| 3.13 | ASGI str (c256) | 759011 | 76044 | 3.363ms | 15.583ms |
| 3.13 | ASGI echo (c512) | 470398 | 47177 | 10.833ms | 39.879ms |
| 3.13 | WSGI bytes (c64) | 630754 | 63075 | 1.014ms | 1.692ms |
| 3.13 | WSGI str (c128) | 622884 | 62324 | 2.052ms | 17.309ms |
| 3.13 | WSGI echo (c128) | 587677 | 58790 | 2.175ms | 15.24ms |
