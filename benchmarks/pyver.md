# Granian benchmarks



## Python versions

Run at: Tue 07 Apr 2026, 12:00    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.12.77 (CPUs: 16)    
Granian version: 2.7.3    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.10 | RSGI get 1KB (c128) | 1464724 | 146423 | 0.87ms | 63.541ms |
| 3.10 | RSGI echo 1KB (c128) | 1228389 | 122802 | 1.038ms | 49.084ms |
| 3.10 | RSGI echo 100KB (iter) (c64) | 174686 | 17464 | 3.649ms | 65.258ms |
| 3.10 | ASGI get 1KB (c128) | 1364184 | 136396 | 0.933ms | 78.097ms |
| 3.10 | ASGI echo 1KB (c128) | 928558 | 92840 | 1.374ms | 52.495ms |
| 3.10 | ASGI echo 100KB (iter) (c64) | 197123 | 19714 | 3.236ms | 52.944ms |
| 3.10 | WSGI get 1KB (c64) | 1465680 | 146507 | 0.434ms | 57.214ms |
| 3.10 | WSGI echo 1KB (c64) | 1360769 | 136040 | 0.468ms | 28.911ms |
| 3.10 | WSGI echo 100KB (iter) (c64) | 106339 | 10638 | 6.006ms | 30.753ms |
| 3.11 | RSGI get 1KB (c128) | 1426224 | 142584 | 0.894ms | 57.258ms |
| 3.11 | RSGI echo 1KB (c128) | 1269281 | 126888 | 1.003ms | 89.544ms |
| 3.11 | RSGI echo 100KB (iter) (c64) | 173959 | 17399 | 3.669ms | 41.42ms |
| 3.11 | ASGI get 1KB (c128) | 1429886 | 142960 | 0.891ms | 70.234ms |
| 3.11 | ASGI echo 1KB (c128) | 986254 | 98607 | 1.292ms | 73.36ms |
| 3.11 | ASGI echo 100KB (iter) (c64) | 193492 | 19350 | 3.3ms | 34.341ms |
| 3.11 | WSGI get 1KB (c64) | 1478767 | 147843 | 0.431ms | 37.916ms |
| 3.11 | WSGI echo 1KB (c64) | 1366978 | 136658 | 0.466ms | 40.976ms |
| 3.11 | WSGI echo 100KB (iter) (c64) | 110278 | 11031 | 5.786ms | 52.689ms |
| 3.12 | RSGI get 1KB (c128) | 1411805 | 141137 | 0.903ms | 45.232ms |
| 3.12 | RSGI echo 1KB (c128) | 1239244 | 123888 | 1.028ms | 72.854ms |
| 3.12 | RSGI echo 100KB (iter) (c64) | 170113 | 17012 | 3.755ms | 27.976ms |
| 3.12 | ASGI get 1KB (c128) | 1393781 | 139337 | 0.915ms | 50.526ms |
| 3.12 | ASGI echo 1KB (c128) | 987359 | 98717 | 1.293ms | 45.948ms |
| 3.12 | ASGI echo 100KB (iter) (c64) | 200682 | 20071 | 3.181ms | 33.625ms |
| 3.12 | WSGI get 1KB (c64) | 1412022 | 141148 | 0.451ms | 42.767ms |
| 3.12 | WSGI echo 1KB (c64) | 1351869 | 135146 | 0.471ms | 46.938ms |
| 3.12 | WSGI echo 100KB (iter) (c64) | 109533 | 10957 | 5.824ms | 56.481ms |
| 3.13 | RSGI get 1KB (c128) | 1429798 | 142955 | 0.892ms | 48.694ms |
| 3.13 | RSGI echo 1KB (c128) | 1247500 | 124727 | 1.022ms | 64.238ms |
| 3.13 | RSGI echo 100KB (iter) (c64) | 172660 | 17267 | 3.695ms | 48.772ms |
| 3.13 | ASGI get 1KB (c128) | 1393855 | 139355 | 0.913ms | 90.504ms |
| 3.13 | ASGI echo 1KB (c128) | 1004056 | 100386 | 1.27ms | 61.491ms |
| 3.13 | ASGI echo 100KB (iter) (c64) | 189581 | 18960 | 3.367ms | 42.057ms |
| 3.13 | WSGI get 1KB (c64) | 1473358 | 147280 | 0.432ms | 52.608ms |
| 3.13 | WSGI echo 1KB (c64) | 1351223 | 135073 | 0.471ms | 53.595ms |
| 3.13 | WSGI echo 100KB (iter) (c64) | 109441 | 10948 | 5.826ms | 62.867ms |
| 3.14 | RSGI get 1KB (c128) | 1476074 | 147569 | 0.864ms | 62.446ms |
| 3.14 | RSGI echo 1KB (c128) | 1231548 | 123123 | 1.032ms | 97.947ms |
| 3.14 | RSGI echo 100KB (iter) (c64) | 173432 | 17344 | 3.681ms | 37.39ms |
| 3.14 | ASGI get 1KB (c128) | 1439442 | 143922 | 0.886ms | 64.139ms |
| 3.14 | ASGI echo 1KB (c128) | 1009738 | 100956 | 1.263ms | 57.633ms |
| 3.14 | ASGI echo 100KB (iter) (c64) | 198846 | 19886 | 3.209ms | 48.322ms |
| 3.14 | WSGI get 1KB (c64) | 1471359 | 147071 | 0.433ms | 54.568ms |
| 3.14 | WSGI echo 1KB (c64) | 1365044 | 136453 | 0.466ms | 48.155ms |
| 3.14 | WSGI echo 100KB (iter) (c64) | 109161 | 10920 | 5.85ms | 34.431ms |
