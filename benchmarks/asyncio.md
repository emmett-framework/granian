# Granian benchmarks



## AsyncIO-specific benchmarks

Run at: Fri 11 Apr 2025, 14:24    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Granian version: 2.2.4

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

### Event loops

Comparison between different AsyncIO event loops on async protocols.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| ASGI asyncio get 10KB (c512) | 815174 | 82061 | 6.23ms | 52.911ms |
| ASGI asyncio echo 10KB (iter) (c128) | 293140 | 29339 | 4.352ms | 29.231ms |
| ASGI rloop get 10KB (c512) | 823575 | 82845 | 6.172ms | 91.785ms |
| ASGI rloop echo 10KB (iter) (c64) | 338508 | 33854 | 1.887ms | 3.282ms |
| ASGI uvloop get 10KB (c128) | 1186778 | 118725 | 1.077ms | 28.681ms |
| ASGI uvloop echo 10KB (iter) (c256) | 594360 | 59601 | 4.289ms | 34.657ms |
| RSGI asyncio get 10KB (c64) | 730922 | 73096 | 0.874ms | 2.223ms |
| RSGI asyncio echo 10KB (iter) (c64) | 282901 | 28290 | 2.26ms | 4.276ms |
| RSGI rloop get 10KB (c64) | 1161599 | 116168 | 0.55ms | 3.207ms |
| RSGI rloop echo 10KB (iter) (c64) | 369963 | 36997 | 1.728ms | 3.042ms |
| RSGI uvloop get 10KB (c128) | 1197676 | 119859 | 1.065ms | 22.936ms |
| RSGI uvloop echo 10KB (iter) (c128) | 756625 | 75756 | 1.684ms | 24.386ms |

### Task implementation

Comparison between Granian Rust AsyncIO task implementation and stdlib one on ASGI protocol.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.9 | asyncio get 10KB (c128) | 779123 | 77946 | 1.64ms | 25.414ms |
| 3.9 | asyncio echo 10KB (iter) (c128) | 291484 | 29178 | 4.377ms | 23.21ms |
| 3.9 | rust get 10KB (c128) | 1079151 | 108002 | 1.182ms | 20.779ms |
| 3.9 | rust echo 10KB (iter) (c256) | 264416 | 26511 | 9.628ms | 653.007ms |
| 3.10 | asyncio get 10KB (c256) | 794423 | 79701 | 3.207ms | 28.229ms |
| 3.10 | asyncio echo 10KB (iter) (c256) | 289659 | 29036 | 8.791ms | 56.503ms |
| 3.10 | rust get 10KB (c128) | 1073708 | 107431 | 1.19ms | 23.555ms |
| 3.10 | rust echo 10KB (iter) (c512) | 254497 | 25570 | 19.978ms | 812.325ms |
| 3.11 | asyncio get 10KB (c512) | 816640 | 82053 | 6.23ms | 47.046ms |
| 3.11 | asyncio echo 10KB (iter) (c128) | 300309 | 30059 | 4.251ms | 15.941ms |
| 3.11 | rust get 10KB (c512) | 1059946 | 106507 | 4.801ms | 93.069ms |
| 3.11 | rust echo 10KB (iter) (c128) | 280391 | 28066 | 4.552ms | 478.563ms |
