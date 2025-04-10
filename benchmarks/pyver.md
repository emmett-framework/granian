# Granian benchmarks



## Python versions

Run at: Thu 10 Apr 2025, 18:15    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Granian version: 2.2.4    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.9 | RSGI get 1KB (c64) | 1305903 | 130590 | 0.489ms | 2.055ms |
| 3.9 | RSGI echo 1KB (c64) | 1179927 | 118002 | 0.541ms | 2.858ms |
| 3.9 | RSGI echo 100KB (iter) (c128) | 194237 | 19426 | 6.571ms | 33.574ms |
| 3.9 | ASGI get 1KB (c512) | 1252464 | 125706 | 4.059ms | 114.147ms |
| 3.9 | ASGI echo 1KB (c128) | 856450 | 85675 | 1.492ms | 11.78ms |
| 3.9 | ASGI echo 100KB (iter) (c64) | 300578 | 30058 | 2.125ms | 9.151ms |
| 3.9 | WSGI get 1KB (c256) | 1295993 | 129981 | 1.968ms | 25.428ms |
| 3.9 | WSGI echo 1KB (c256) | 1228282 | 123222 | 2.075ms | 26.878ms |
| 3.9 | WSGI echo 100KB (iter) (c64) | 91180 | 9119 | 6.998ms | 23.155ms |
| 3.10 | RSGI get 1KB (c64) | 1295170 | 129524 | 0.493ms | 1.774ms |
| 3.10 | RSGI echo 1KB (c128) | 1216168 | 121734 | 1.048ms | 20.572ms |
| 3.10 | RSGI echo 100KB (iter) (c128) | 192873 | 19301 | 6.61ms | 35.656ms |
| 3.10 | ASGI get 1KB (c128) | 1257486 | 125844 | 1.014ms | 27.812ms |
| 3.10 | ASGI echo 1KB (c128) | 878653 | 87912 | 1.452ms | 18.827ms |
| 3.10 | ASGI echo 100KB (iter) (c64) | 307526 | 30750 | 2.078ms | 10.513ms |
| 3.10 | WSGI get 1KB (c64) | 1306105 | 130611 | 0.489ms | 1.753ms |
| 3.10 | WSGI echo 1KB (c128) | 1232107 | 123256 | 1.036ms | 19.507ms |
| 3.10 | WSGI echo 100KB (iter) (c64) | 92936 | 9294 | 6.871ms | 23.236ms |
| 3.11 | RSGI get 1KB (c256) | 796988 | 79838 | 3.203ms | 26.794ms |
| 3.11 | RSGI echo 1KB (c256) | 566176 | 56718 | 4.504ms | 21.234ms |
| 3.11 | RSGI echo 100KB (iter) (c64) | 201637 | 20165 | 3.167ms | 10.053ms |
| 3.11 | ASGI get 1KB (c128) | 637318 | 63819 | 2.002ms | 24.499ms |
| 3.11 | ASGI echo 1KB (c64) | 464695 | 46476 | 1.373ms | 2.701ms |
| 3.11 | ASGI echo 100KB (iter) (c64) | 225056 | 22507 | 2.839ms | 9.909ms |
| 3.11 | WSGI get 1KB (c256) | 703502 | 70506 | 3.628ms | 15.493ms |
| 3.11 | WSGI echo 1KB (c128) | 650897 | 65125 | 1.962ms | 9.756ms |
| 3.11 | WSGI echo 100KB (iter) (c64) | 70568 | 7057 | 9.052ms | 27.427ms |
| 3.12 | RSGI get 1KB (c256) | 790639 | 79291 | 3.224ms | 26.651ms |
| 3.12 | RSGI echo 1KB (c128) | 557299 | 55749 | 2.29ms | 22.579ms |
| 3.12 | RSGI echo 100KB (iter) (c128) | 169205 | 16926 | 7.539ms | 36.854ms |
| 3.12 | ASGI get 1KB (c128) | 657527 | 65794 | 1.943ms | 26.119ms |
| 3.12 | ASGI echo 1KB (c64) | 451893 | 45190 | 1.414ms | 2.564ms |
| 3.12 | ASGI echo 100KB (iter) (c128) | 171192 | 17133 | 7.443ms | 38.736ms |
| 3.12 | WSGI get 1KB (c64) | 671060 | 67115 | 0.951ms | 1.86ms |
| 3.12 | WSGI echo 1KB (c512) | 587995 | 59073 | 8.637ms | 83.268ms |
| 3.12 | WSGI echo 100KB (iter) (c64) | 70425 | 7043 | 9.059ms | 29.615ms |
| 3.13 | RSGI get 1KB (c256) | 809461 | 81157 | 3.149ms | 26.443ms |
| 3.13 | RSGI echo 1KB (c128) | 585439 | 58584 | 2.182ms | 22.178ms |
| 3.13 | RSGI echo 100KB (iter) (c128) | 172213 | 17240 | 7.395ms | 38.089ms |
| 3.13 | ASGI get 1KB (c256) | 629663 | 63158 | 4.046ms | 40.151ms |
| 3.13 | ASGI echo 1KB (c128) | 447574 | 44797 | 2.847ms | 25.577ms |
| 3.13 | ASGI echo 100KB (iter) (c64) | 171771 | 17178 | 3.716ms | 9.217ms |
| 3.13 | WSGI get 1KB (c64) | 658028 | 65808 | 0.971ms | 1.725ms |
| 3.13 | WSGI echo 1KB (c64) | 610645 | 61066 | 1.047ms | 2.471ms |
| 3.13 | WSGI echo 100KB (iter) (c64) | 69604 | 6960 | 9.178ms | 29.163ms |
