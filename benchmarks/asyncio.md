# Granian benchmarks



## AsyncIO-specific benchmarks

Run at: Thu 10 Apr 2025, 19:04    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Granian version: 2.2.4

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

### Event loops

Comparison between different AsyncIO event loops on async protocols.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| ASGI asyncio get 10KB (c256) | 432435 | 43321 | 5.9ms | 26.207ms |
| ASGI asyncio echo 10KB (iter) (c512) | 158995 | 15973 | 31.966ms | 87.138ms |
| ASGI rloop get 10KB (c128) | 483499 | 48389 | 2.641ms | 28.655ms |
| ASGI rloop echo 10KB (iter) (c256) | 212595 | 21321 | 11.978ms | 35.009ms |
| ASGI uvloop get 10KB (c128) | 603891 | 60473 | 2.113ms | 30.854ms |
| ASGI uvloop echo 10KB (iter) (c64) | 324471 | 32449 | 1.969ms | 3.866ms |
| RSGI asyncio get 10KB (c256) | 549522 | 55109 | 4.637ms | 33.726ms |
| RSGI asyncio echo 10KB (iter) (c512) | 177656 | 17833 | 28.608ms | 67.882ms |
| RSGI rloop get 10KB (c256) | 581998 | 58367 | 4.379ms | 30.82ms |
| RSGI rloop echo 10KB (iter) (c128) | 244822 | 24520 | 5.206ms | 29.96ms |
| RSGI uvloop get 10KB (c256) | 761961 | 76397 | 3.346ms | 28.591ms |
| RSGI uvloop echo 10KB (iter) (c128) | 390977 | 39117 | 3.269ms | 25.955ms |

### Task implementation

Comparison between Granian Rust AsyncIO task implementation and stdlib one on ASGI protocol.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.9 | asyncio get 10KB (c256) | 761286 | 76376 | 3.348ms | 21.288ms |
| 3.9 | asyncio echo 10KB (iter) (c256) | 292569 | 29333 | 8.711ms | 33.948ms |
| 3.9 | rust get 10KB (c128) | 1035338 | 103646 | 1.233ms | 22.507ms |
| 3.9 | rust echo 10KB (iter) (c256) | 259011 | 25960 | 9.843ms | 646.772ms |
| 3.10 | asyncio get 10KB (c128) | 790364 | 79112 | 1.615ms | 29.36ms |
| 3.10 | asyncio echo 10KB (iter) (c128) | 285253 | 28561 | 4.473ms | 29.524ms |
| 3.10 | rust get 10KB (c256) | 1107680 | 111084 | 2.302ms | 27.872ms |
| 3.10 | rust echo 10KB (iter) (c128) | 255740 | 25596 | 4.985ms | 448.22ms |
| 3.11 | asyncio get 10KB (c256) | 446617 | 44780 | 5.708ms | 51.82ms |
| 3.11 | asyncio echo 10KB (iter) (c512) | 159907 | 16082 | 31.724ms | 63.638ms |
| 3.11 | rust get 10KB (c256) | 516154 | 51735 | 4.942ms | 30.622ms |
| 3.11 | rust echo 10KB (iter) (c256) | 153283 | 15371 | 16.18ms | 365.296ms |
