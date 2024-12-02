# Granian benchmarks



## Python versions

Run at: Mon 02 Dec 2024, 00:01    
Environment: GHA Linux x86_64 (CPUs: 4)    
Granian version: 1.7.0    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.10 | RSGI bytes (c512) | 457389 | 45967 | 11.076ms | 160.533ms |
| 3.10 | RSGI str (c64) | 430947 | 43100 | 1.481ms | 3.747ms |
| 3.10 | RSGI echo (c512) | 364376 | 36617 | 13.913ms | 108.031ms |
| 3.10 | ASGI bytes (c64) | 445369 | 44547 | 1.432ms | 3.941ms |
| 3.10 | ASGI str (c256) | 419963 | 42136 | 6.045ms | 65.916ms |
| 3.10 | ASGI echo (c64) | 325700 | 32576 | 1.96ms | 3.593ms |
| 3.10 | WSGI bytes (c256) | 541239 | 54263 | 4.699ms | 54.037ms |
| 3.10 | WSGI str (c512) | 562159 | 56529 | 8.999ms | 142.685ms |
| 3.10 | WSGI echo (c128) | 488292 | 48891 | 2.608ms | 32.309ms |
| 3.11 | RSGI bytes (c512) | 352007 | 35362 | 14.396ms | 156.357ms |
| 3.11 | RSGI str (c512) | 346662 | 34879 | 14.586ms | 134.112ms |
| 3.11 | RSGI echo (c128) | 204575 | 20481 | 6.229ms | 19.538ms |
| 3.11 | ASGI bytes (c256) | 295881 | 29672 | 8.581ms | 65.616ms |
| 3.11 | ASGI str (c256) | 292597 | 29341 | 8.681ms | 69.617ms |
| 3.11 | ASGI echo (c512) | 184777 | 18554 | 27.432ms | 98.691ms |
| 3.11 | WSGI bytes (c64) | 377301 | 37738 | 1.691ms | 4.285ms |
| 3.11 | WSGI str (c64) | 373658 | 37373 | 1.708ms | 4.014ms |
| 3.11 | WSGI echo (c64) | 348390 | 34845 | 1.832ms | 4.375ms |
| 3.12 | RSGI bytes (c256) | 337191 | 33812 | 7.532ms | 75.211ms |
| 3.12 | RSGI str (c256) | 331836 | 33300 | 7.648ms | 73.752ms |
| 3.12 | RSGI echo (c128) | 199286 | 19959 | 6.385ms | 27.576ms |
| 3.12 | ASGI bytes (c256) | 284989 | 28563 | 8.923ms | 63.583ms |
| 3.12 | ASGI str (c256) | 282949 | 28372 | 8.975ms | 57.835ms |
| 3.12 | ASGI echo (c128) | 177369 | 17754 | 7.187ms | 20.041ms |
| 3.12 | WSGI bytes (c256) | 347995 | 34904 | 7.302ms | 59.784ms |
| 3.12 | WSGI str (c256) | 338902 | 33985 | 7.496ms | 65.797ms |
| 3.12 | WSGI echo (c128) | 321616 | 32199 | 3.957ms | 35.946ms |
| 3.13 | RSGI bytes (c512) | 352564 | 35448 | 14.373ms | 114.659ms |
| 3.13 | RSGI str (c512) | 374322 | 37595 | 13.546ms | 121.957ms |
| 3.13 | RSGI echo (c512) | 213039 | 21416 | 23.742ms | 124.886ms |
| 3.13 | ASGI bytes (c512) | 289828 | 29116 | 17.488ms | 127.387ms |
| 3.13 | ASGI str (c256) | 290817 | 29174 | 8.737ms | 71.287ms |
| 3.13 | ASGI echo (c128) | 180730 | 18101 | 7.036ms | 40.868ms |
| 3.13 | WSGI bytes (c128) | 368149 | 36865 | 3.457ms | 31.9ms |
| 3.13 | WSGI str (c256) | 357390 | 35870 | 7.097ms | 70.008ms |
| 3.13 | WSGI echo (c128) | 353821 | 35426 | 3.597ms | 40.647ms |
