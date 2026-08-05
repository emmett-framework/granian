# Granian benchmarks



## Python versions

Run at: Wed 05 Aug 2026, 17:19    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.18.41 (CPUs: 16)    
Granian version: 2.8.1    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.10 | RSGI get 1KB (c128) | 1447426 | 144711 | 0.88ms | 59.84ms |
| 3.10 | RSGI echo 1KB (c128) | 1158772 | 115850 | 1.1ms | 72.829ms |
| 3.10 | RSGI echo 100KB (iter) (c64) | 173011 | 17305 | 3.688ms | 43.182ms |
| 3.10 | ASGI get 1KB (c128) | 1350388 | 135015 | 0.944ms | 50.364ms |
| 3.10 | ASGI echo 1KB (c128) | 961337 | 96118 | 1.327ms | 50.077ms |
| 3.10 | ASGI echo 100KB (iter) (c64) | 180610 | 18063 | 3.535ms | 34.777ms |
| 3.10 | WSGI get 1KB (c64) | 1450385 | 144990 | 0.439ms | 34.815ms |
| 3.10 | WSGI echo 1KB (c64) | 1370662 | 137018 | 0.465ms | 34.356ms |
| 3.10 | WSGI echo 100KB (iter) (c64) | 97694 | 9774 | 6.537ms | 36.237ms |
| 3.11 | RSGI get 1KB (c128) | 1431681 | 143137 | 0.89ms | 74.088ms |
| 3.11 | RSGI echo 1KB (c128) | 1177824 | 117759 | 1.083ms | 50.972ms |
| 3.11 | RSGI echo 100KB (iter) (c64) | 173241 | 17327 | 3.683ms | 43.735ms |
| 3.11 | ASGI get 1KB (c128) | 1422595 | 142233 | 0.896ms | 64.465ms |
| 3.11 | ASGI echo 1KB (c128) | 1014398 | 101417 | 1.257ms | 54.049ms |
| 3.11 | ASGI echo 100KB (iter) (c64) | 184083 | 18410 | 3.468ms | 37.3ms |
| 3.11 | WSGI get 1KB (c64) | 1425962 | 142556 | 0.447ms | 33.148ms |
| 3.11 | WSGI echo 1KB (c64) | 1379814 | 137943 | 0.462ms | 19.489ms |
| 3.11 | WSGI echo 100KB (iter) (c64) | 101446 | 10148 | 6.295ms | 36.685ms |
| 3.12 | RSGI get 1KB (c128) | 1464944 | 146461 | 0.87ms | 55.526ms |
| 3.12 | RSGI echo 1KB (c128) | 1202877 | 120263 | 1.06ms | 53.824ms |
| 3.12 | RSGI echo 100KB (iter) (c64) | 169615 | 16964 | 3.765ms | 31.408ms |
| 3.12 | ASGI get 1KB (c128) | 1432089 | 143167 | 0.89ms | 46.981ms |
| 3.12 | ASGI echo 1KB (c128) | 1018862 | 101860 | 1.251ms | 64.662ms |
| 3.12 | ASGI echo 100KB (iter) (c64) | 189927 | 18996 | 3.361ms | 35.189ms |
| 3.12 | WSGI get 1KB (c64) | 1430193 | 142974 | 0.445ms | 37.08ms |
| 3.12 | WSGI echo 1KB (c64) | 1361318 | 136079 | 0.468ms | 27.641ms |
| 3.12 | WSGI echo 100KB (iter) (c64) | 100460 | 10051 | 6.355ms | 41.213ms |
| 3.13 | RSGI get 1KB (c128) | 1461792 | 146147 | 0.873ms | 40.13ms |
| 3.13 | RSGI echo 1KB (c128) | 1207486 | 120722 | 1.057ms | 49.578ms |
| 3.13 | RSGI echo 100KB (iter) (c64) | 176713 | 17675 | 3.612ms | 38.66ms |
| 3.13 | ASGI get 1KB (c128) | 1408150 | 140789 | 0.905ms | 44.952ms |
| 3.13 | ASGI echo 1KB (c128) | 987318 | 98714 | 1.29ms | 86.366ms |
| 3.13 | ASGI echo 100KB (iter) (c64) | 180888 | 18091 | 3.527ms | 43.321ms |
| 3.13 | WSGI get 1KB (c64) | 1471537 | 147100 | 0.433ms | 32.691ms |
| 3.13 | WSGI echo 1KB (c64) | 1381911 | 138152 | 0.461ms | 18.706ms |
| 3.13 | WSGI echo 100KB (iter) (c64) | 100762 | 10081 | 6.339ms | 29.331ms |
| 3.14 | RSGI get 1KB (c128) | 1458717 | 145844 | 0.875ms | 34.695ms |
| 3.14 | RSGI echo 1KB (c128) | 1243321 | 124313 | 1.025ms | 55.603ms |
| 3.14 | RSGI echo 100KB (iter) (c64) | 172276 | 17231 | 3.705ms | 36.607ms |
| 3.14 | ASGI get 1KB (c128) | 1465860 | 146554 | 0.87ms | 59.518ms |
| 3.14 | ASGI echo 1KB (c128) | 1069854 | 106970 | 1.191ms | 71.902ms |
| 3.14 | ASGI echo 100KB (iter) (c64) | 187619 | 18763 | 3.401ms | 42.884ms |
| 3.14 | WSGI get 1KB (c64) | 1463202 | 146271 | 0.435ms | 33.432ms |
| 3.14 | WSGI echo 1KB (c64) | 1378211 | 137766 | 0.463ms | 19.269ms |
| 3.14 | WSGI echo 100KB (iter) (c64) | 101293 | 10133 | 6.303ms | 42.794ms |
