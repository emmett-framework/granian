# Granian benchmarks



## Python versions

Run at: Fri 11 Apr 2025, 13:35    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Granian version: 2.2.4    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.9 | RSGI get 1KB (c128) | 1280221 | 128063 | 0.998ms | 11.678ms |
| 3.9 | RSGI echo 1KB (c64) | 1219245 | 121930 | 0.524ms | 1.632ms |
| 3.9 | RSGI echo 100KB (iter) (c64) | 288514 | 28847 | 2.216ms | 10.141ms |
| 3.9 | ASGI get 1KB (c128) | 1267646 | 126806 | 1.009ms | 12.691ms |
| 3.9 | ASGI echo 1KB (c128) | 929863 | 93019 | 1.374ms | 12.381ms |
| 3.9 | ASGI echo 100KB (iter) (c64) | 308548 | 30853 | 2.071ms | 9.808ms |
| 3.9 | WSGI get 1KB (c64) | 1279995 | 128009 | 0.499ms | 1.694ms |
| 3.9 | WSGI echo 1KB (c128) | 1239727 | 124027 | 1.029ms | 20.935ms |
| 3.9 | WSGI echo 100KB (iter) (c64) | 91737 | 9174 | 6.96ms | 21.801ms |
| 3.10 | RSGI get 1KB (c128) | 1297157 | 129799 | 0.985ms | 22.793ms |
| 3.10 | RSGI echo 1KB (c256) | 1229070 | 123042 | 2.075ms | 33.152ms |
| 3.10 | RSGI echo 100KB (iter) (c128) | 197517 | 19774 | 6.449ms | 40.745ms |
| 3.10 | ASGI get 1KB (c256) | 1258589 | 126210 | 2.026ms | 26.31ms |
| 3.10 | ASGI echo 1KB (c128) | 944785 | 94510 | 1.352ms | 12.699ms |
| 3.10 | ASGI echo 100KB (iter) (c64) | 307574 | 30754 | 2.077ms | 10.126ms |
| 3.10 | WSGI get 1KB (c64) | 1311542 | 131150 | 0.487ms | 2.265ms |
| 3.10 | WSGI echo 1KB (c64) | 1253595 | 125359 | 0.51ms | 2.271ms |
| 3.10 | WSGI echo 100KB (iter) (c64) | 93740 | 9374 | 6.818ms | 20.768ms |
| 3.11 | RSGI get 1KB (c256) | 1273668 | 127577 | 2.004ms | 25.745ms |
| 3.11 | RSGI echo 1KB (c128) | 1224102 | 122548 | 1.043ms | 24.258ms |
| 3.11 | RSGI echo 100KB (iter) (c64) | 245351 | 24535 | 2.601ms | 11.438ms |
| 3.11 | ASGI get 1KB (c128) | 1284499 | 128566 | 0.994ms | 26.21ms |
| 3.11 | ASGI echo 1KB (c128) | 975070 | 97596 | 1.308ms | 20.738ms |
| 3.11 | ASGI echo 100KB (iter) (c64) | 308173 | 30815 | 2.073ms | 10.459ms |
| 3.11 | WSGI get 1KB (c64) | 1312843 | 131288 | 0.487ms | 1.898ms |
| 3.11 | WSGI echo 1KB (c64) | 1255488 | 125554 | 0.509ms | 1.782ms |
| 3.11 | WSGI echo 100KB (iter) (c64) | 91666 | 9167 | 6.966ms | 20.412ms |
| 3.12 | RSGI get 1KB (c64) | 1278446 | 127852 | 0.5ms | 1.641ms |
| 3.12 | RSGI echo 1KB (c256) | 1225917 | 122972 | 2.074ms | 46.151ms |
| 3.12 | RSGI echo 100KB (iter) (c64) | 288368 | 28834 | 2.217ms | 10.12ms |
| 3.12 | ASGI get 1KB (c128) | 1254146 | 125575 | 1.017ms | 22.776ms |
| 3.12 | ASGI echo 1KB (c128) | 878391 | 87890 | 1.453ms | 20.092ms |
| 3.12 | ASGI echo 100KB (iter) (c64) | 205891 | 20589 | 3.103ms | 10.725ms |
| 3.12 | WSGI get 1KB (c64) | 1292466 | 129261 | 0.494ms | 1.741ms |
| 3.12 | WSGI echo 1KB (c64) | 1218864 | 121890 | 0.524ms | 1.639ms |
| 3.12 | WSGI echo 100KB (iter) (c64) | 91525 | 9153 | 6.972ms | 23.179ms |
| 3.13 | RSGI get 1KB (c128) | 1281870 | 128237 | 0.997ms | 27.672ms |
| 3.13 | RSGI echo 1KB (c128) | 1221200 | 122233 | 1.044ms | 26.67ms |
| 3.13 | RSGI echo 100KB (iter) (c128) | 201255 | 20133 | 6.336ms | 38.87ms |
| 3.13 | ASGI get 1KB (c256) | 1257925 | 126045 | 2.029ms | 19.76ms |
| 3.13 | ASGI echo 1KB (c128) | 944970 | 94597 | 1.351ms | 27.446ms |
| 3.13 | ASGI echo 100KB (iter) (c64) | 307398 | 30737 | 2.079ms | 10.172ms |
| 3.13 | WSGI get 1KB (c64) | 1312649 | 131277 | 0.486ms | 1.667ms |
| 3.13 | WSGI echo 1KB (c64) | 1265714 | 126567 | 0.505ms | 1.589ms |
| 3.13 | WSGI echo 100KB (iter) (c64) | 89574 | 8958 | 7.128ms | 21.567ms |
