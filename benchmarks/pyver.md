# Granian benchmarks



## Python versions

Run at: Mon 28 Oct 2024, 02:17    
Environment: GHA Linux x86_64 (CPUs: 4)    
Granian version: 1.6.2    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.10 | RSGI bytes (c64) | 445347 | 44540 | 1.433ms | 4.16ms |
| 3.10 | RSGI str (c64) | 407154 | 40713 | 1.569ms | 4.146ms |
| 3.10 | RSGI echo (c64) | 409490 | 40952 | 1.559ms | 3.98ms |
| 3.10 | ASGI bytes (c64) | 404741 | 40475 | 1.577ms | 3.853ms |
| 3.10 | ASGI str (c64) | 443742 | 44379 | 1.439ms | 3.743ms |
| 3.10 | ASGI echo (c64) | 288973 | 28904 | 2.208ms | 4.757ms |
| 3.10 | WSGI bytes (c128) | 559148 | 55978 | 2.28ms | 24.538ms |
| 3.10 | WSGI str (c256) | 567857 | 56935 | 4.476ms | 66.155ms |
| 3.10 | WSGI echo (c512) | 501010 | 50392 | 10.098ms | 117.374ms |
| 3.11 | RSGI bytes (c64) | 408906 | 40897 | 1.56ms | 3.754ms |
| 3.11 | RSGI str (c64) | 444267 | 44427 | 1.437ms | 4.131ms |
| 3.11 | RSGI echo (c256) | 375214 | 37628 | 6.77ms | 69.911ms |
| 3.11 | ASGI bytes (c128) | 394648 | 39496 | 3.232ms | 19.781ms |
| 3.11 | ASGI str (c512) | 416410 | 41848 | 12.182ms | 104.538ms |
| 3.11 | ASGI echo (c64) | 220900 | 22093 | 2.888ms | 5.823ms |
| 3.11 | WSGI bytes (c128) | 381549 | 38197 | 3.339ms | 24.674ms |
| 3.11 | WSGI str (c128) | 377113 | 37754 | 3.377ms | 32.506ms |
| 3.11 | WSGI echo (c256) | 342484 | 34363 | 7.42ms | 66.93ms |
| 3.12 | RSGI bytes (c512) | 423560 | 42590 | 11.958ms | 141.381ms |
| 3.12 | RSGI str (c512) | 427772 | 43035 | 11.842ms | 114.818ms |
| 3.12 | RSGI echo (c512) | 361753 | 36365 | 13.995ms | 169.242ms |
| 3.12 | ASGI bytes (c128) | 419544 | 42014 | 3.032ms | 36.038ms |
| 3.12 | ASGI str (c64) | 415682 | 41567 | 1.536ms | 3.97ms |
| 3.12 | ASGI echo (c64) | 212445 | 21248 | 3.004ms | 6.795ms |
| 3.12 | WSGI bytes (c256) | 356792 | 35798 | 7.121ms | 64.292ms |
| 3.12 | WSGI str (c128) | 362003 | 36244 | 3.52ms | 20.129ms |
| 3.12 | WSGI echo (c512) | 322632 | 32432 | 15.676ms | 144.224ms |
| 3.13 | RSGI bytes (c512) | 400784 | 40264 | 12.635ms | 175.762ms |
| 3.13 | RSGI str (c64) | 407448 | 40745 | 1.568ms | 4.455ms |
| 3.13 | RSGI echo (c128) | 384481 | 38485 | 3.315ms | 23.927ms |
| 3.13 | ASGI bytes (c512) | 414131 | 41618 | 12.238ms | 147.581ms |
| 3.13 | ASGI str (c64) | 449520 | 44963 | 1.419ms | 3.919ms |
| 3.13 | ASGI echo (c128) | 225465 | 22567 | 5.648ms | 36.339ms |
| 3.13 | WSGI bytes (c128) | 371324 | 37194 | 3.429ms | 18.313ms |
| 3.13 | WSGI str (c64) | 369008 | 36910 | 1.729ms | 5.06ms |
| 3.13 | WSGI echo (c128) | 363711 | 36408 | 3.505ms | 20.864ms |
