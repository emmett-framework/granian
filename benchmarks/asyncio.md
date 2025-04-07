# Granian benchmarks



## AsyncIO-specific benchmarks

Run at: Mon 07 Apr 2025, 13:16    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Granian version: 2.2.2

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

### Event loops

Comparison between different AsyncIO event loops on async protocols.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| ASGI asyncio get 10KB (c256) | 431465 | 43243 | 5.907ms | 42.3ms |
| ASGI asyncio echo 10KB (iter) (c256) | 127656 | 12793 | 19.907ms | 69.41ms |
| ASGI rloop get 10KB (c512) | 484289 | 48610 | 10.51ms | 65.384ms |
| ASGI rloop echo 10KB (iter) (c256) | 179718 | 18005 | 14.184ms | 39.284ms |
| ASGI uvloop get 10KB (c128) | 618541 | 61895 | 2.065ms | 28.91ms |
| ASGI uvloop echo 10KB (iter) (c64) | 285624 | 28564 | 2.237ms | 4.203ms |
| RSGI asyncio get 10KB (c256) | 543225 | 54441 | 4.695ms | 50.486ms |
| RSGI asyncio echo 10KB (iter) (c256) | 114263 | 11449 | 22.31ms | 36.879ms |
| RSGI rloop get 10KB (c512) | 593758 | 59688 | 8.559ms | 43.597ms |
| RSGI rloop echo 10KB (iter) (c256) | 168903 | 16951 | 15.071ms | 33.323ms |
| RSGI uvloop get 10KB (c256) | 759411 | 76069 | 3.354ms | 45.669ms |
| RSGI uvloop echo 10KB (iter) (c256) | 286322 | 28697 | 8.908ms | 25.108ms |

### Task implementation

Comparison between Granian Rust AsyncIO task implementation and stdlib one on ASGI protocol.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.9 | asyncio get 10KB (c256) | 756676 | 75900 | 3.369ms | 27.574ms |
| 3.9 | asyncio echo 10KB (iter) (c128) | 217436 | 21754 | 5.875ms | 17.743ms |
| 3.9 | rust get 10KB (c128) | 1072243 | 107352 | 1.189ms | 19.734ms |
| 3.9 | rust echo 10KB (iter) (c128) | 188148 | 18839 | 6.781ms | 520.774ms |
| 3.10 | asyncio get 10KB (c256) | 791125 | 79194 | 3.223ms | 36.63ms |
| 3.10 | asyncio echo 10KB (iter) (c128) | 223315 | 22356 | 5.715ms | 30.507ms |
| 3.10 | rust get 10KB (c256) | 1108450 | 111057 | 2.303ms | 18.399ms |
| 3.10 | rust echo 10KB (iter) (c128) | 200026 | 20011 | 6.386ms | 461.701ms |
| 3.11 | asyncio get 10KB (c256) | 440289 | 44182 | 5.784ms | 28.179ms |
| 3.11 | asyncio echo 10KB (iter) (c256) | 131785 | 13210 | 19.315ms | 63.406ms |
| 3.11 | rust get 10KB (c256) | 528502 | 53046 | 4.816ms | 31.307ms |
| 3.11 | rust echo 10KB (iter) (c512) | 125139 | 12569 | 40.602ms | 520.916ms |
