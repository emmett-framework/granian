# Granian benchmarks



## Python versions

Run at: Mon 02 Dec 2024, 00:55    
Environment: GHA Linux x86_64 (CPUs: 4)    
Granian version: 1.7.0    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.10 | RSGI bytes (c512) | 440618 | 44304 | 11.506ms | 107.375ms |
| 3.10 | RSGI str (c512) | 421202 | 42364 | 12.016ms | 162.295ms |
| 3.10 | RSGI echo (c256) | 444067 | 44544 | 5.718ms | 66.955ms |
| 3.10 | ASGI bytes (c64) | 480071 | 48014 | 1.33ms | 4.327ms |
| 3.10 | ASGI str (c128) | 463059 | 46351 | 2.752ms | 32.446ms |
| 3.10 | ASGI echo (c256) | 322516 | 32338 | 7.876ms | 62.12ms |
| 3.10 | WSGI bytes (c64) | 581855 | 58197 | 1.097ms | 3.706ms |
| 3.10 | WSGI str (c128) | 587753 | 58847 | 2.166ms | 36.965ms |
| 3.10 | WSGI echo (c256) | 511579 | 51289 | 4.967ms | 68.119ms |
| 3.11 | RSGI bytes (c512) | 361528 | 36347 | 14.003ms | 168.407ms |
| 3.11 | RSGI str (c512) | 353154 | 35491 | 14.348ms | 111.86ms |
| 3.11 | RSGI echo (c128) | 246373 | 24661 | 5.171ms | 27.889ms |
| 3.11 | ASGI bytes (c256) | 294131 | 29490 | 8.643ms | 60.3ms |
| 3.11 | ASGI str (c256) | 295147 | 29606 | 8.6ms | 78.267ms |
| 3.11 | ASGI echo (c64) | 180768 | 18080 | 3.532ms | 6.544ms |
| 3.11 | WSGI bytes (c64) | 374282 | 37436 | 1.704ms | 4.776ms |
| 3.11 | WSGI str (c128) | 374844 | 37534 | 3.394ms | 35.962ms |
| 3.11 | WSGI echo (c64) | 349800 | 34986 | 1.824ms | 4.8ms |
| 3.12 | RSGI bytes (c512) | 343958 | 34580 | 14.719ms | 145.696ms |
| 3.12 | RSGI str (c512) | 366367 | 36810 | 13.819ms | 118.651ms |
| 3.12 | RSGI echo (c64) | 241441 | 24149 | 2.645ms | 4.666ms |
| 3.12 | ASGI bytes (c256) | 290317 | 29110 | 8.762ms | 57.248ms |
| 3.12 | ASGI str (c256) | 291116 | 29174 | 8.74ms | 53.59ms |
| 3.12 | ASGI echo (c256) | 179411 | 17994 | 14.141ms | 80.929ms |
| 3.12 | WSGI bytes (c64) | 355178 | 35521 | 1.798ms | 4.674ms |
| 3.12 | WSGI str (c64) | 354859 | 35493 | 1.798ms | 4.471ms |
| 3.12 | WSGI echo (c64) | 332895 | 33298 | 1.916ms | 4.63ms |
| 3.13 | RSGI bytes (c512) | 361624 | 36328 | 14.01ms | 146.021ms |
| 3.13 | RSGI str (c512) | 344628 | 34625 | 14.719ms | 114.716ms |
| 3.13 | RSGI echo (c128) | 249744 | 25002 | 5.104ms | 24.885ms |
| 3.13 | ASGI bytes (c256) | 298625 | 29940 | 8.517ms | 61.733ms |
| 3.13 | ASGI str (c256) | 288495 | 28920 | 8.814ms | 57.863ms |
| 3.13 | ASGI echo (c128) | 189265 | 18944 | 6.735ms | 25.024ms |
| 3.13 | WSGI bytes (c64) | 363759 | 36389 | 1.751ms | 4.084ms |
| 3.13 | WSGI str (c512) | 378694 | 38075 | 13.37ms | 107.109ms |
| 3.13 | WSGI echo (c128) | 364371 | 36490 | 3.493ms | 32.779ms |
