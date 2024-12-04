# Granian benchmarks



## Python versions

Run at: Wed 04 Dec 2024, 18:20    
Environment: GHA Linux x86_64 (CPUs: 4)    
Granian version: 1.7.0    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.10 | RSGI bytes (c512) | 468383 | 47114 | 10.815ms | 113.705ms |
| 3.10 | RSGI str (c512) | 431957 | 43488 | 11.699ms | 184.274ms |
| 3.10 | RSGI echo (c512) | 377897 | 38001 | 13.409ms | 135.782ms |
| 3.10 | ASGI bytes (c256) | 451615 | 45301 | 5.626ms | 67.399ms |
| 3.10 | ASGI str (c256) | 464428 | 46570 | 5.475ms | 60.829ms |
| 3.10 | ASGI echo (c64) | 343715 | 34380 | 1.857ms | 3.994ms |
| 3.10 | WSGI bytes (c256) | 566264 | 56804 | 4.487ms | 60.134ms |
| 3.10 | WSGI str (c512) | 558336 | 56108 | 9.082ms | 112.142ms |
| 3.10 | WSGI echo (c256) | 495808 | 49729 | 5.129ms | 66.182ms |
| 3.11 | RSGI bytes (c512) | 348746 | 35035 | 14.546ms | 101.277ms |
| 3.11 | RSGI str (c512) | 349941 | 35175 | 14.475ms | 135.633ms |
| 3.11 | RSGI echo (c128) | 215304 | 21551 | 5.919ms | 28.388ms |
| 3.11 | ASGI bytes (c512) | 293432 | 29472 | 17.27ms | 106.454ms |
| 3.11 | ASGI str (c256) | 291417 | 29232 | 8.721ms | 64.756ms |
| 3.11 | ASGI echo (c128) | 192387 | 19261 | 6.624ms | 19.449ms |
| 3.11 | WSGI bytes (c256) | 373989 | 37511 | 6.783ms | 84.18ms |
| 3.11 | WSGI str (c128) | 382768 | 38320 | 3.327ms | 27.745ms |
| 3.11 | WSGI echo (c256) | 343747 | 34468 | 7.399ms | 64.026ms |
| 3.12 | RSGI bytes (c512) | 351196 | 35321 | 14.426ms | 115.03ms |
| 3.12 | RSGI str (c512) | 342557 | 34439 | 14.783ms | 133.841ms |
| 3.12 | RSGI echo (c128) | 210195 | 21039 | 6.065ms | 24.788ms |
| 3.12 | ASGI bytes (c256) | 286099 | 28697 | 8.879ms | 58.297ms |
| 3.12 | ASGI str (c256) | 290703 | 29133 | 8.75ms | 52.89ms |
| 3.12 | ASGI echo (c128) | 184721 | 18491 | 6.895ms | 32.933ms |
| 3.12 | WSGI bytes (c512) | 347281 | 34909 | 14.563ms | 147.089ms |
| 3.12 | WSGI str (c256) | 350185 | 35115 | 7.256ms | 67.699ms |
| 3.12 | WSGI echo (c512) | 325936 | 32774 | 15.527ms | 108.844ms |
| 3.13 | RSGI bytes (c512) | 351377 | 35372 | 14.381ms | 174.904ms |
| 3.13 | RSGI str (c512) | 355561 | 35730 | 14.249ms | 144.48ms |
| 3.13 | RSGI echo (c128) | 221847 | 22213 | 5.739ms | 27.72ms |
| 3.13 | ASGI bytes (c256) | 295817 | 29677 | 8.58ms | 74.849ms |
| 3.13 | ASGI str (c256) | 288736 | 28952 | 8.793ms | 77.611ms |
| 3.13 | ASGI echo (c128) | 193014 | 19326 | 6.595ms | 31.385ms |
| 3.13 | WSGI bytes (c128) | 388182 | 38854 | 3.284ms | 20.921ms |
| 3.13 | WSGI str (c128) | 364941 | 36544 | 3.491ms | 24.884ms |
| 3.13 | WSGI echo (c64) | 345669 | 34574 | 1.846ms | 4.962ms |
