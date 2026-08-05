# Granian benchmarks



## AsyncIO-specific benchmarks

Run at: Wed 05 Aug 2026, 17:39    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.18.41 (CPUs: 16)    
Granian version: 2.8.1

Same methodology of the main benchmarks applies.

### Event loops

Comparison between different AsyncIO event loops on async protocols.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| ASGI asyncio get 10KB (c128) | 852931 | 85281 | 1.495ms | 55.368ms |
| ASGI asyncio echo 10KB (iter) (c128) | 327496 | 32751 | 3.895ms | 69.804ms |
| ASGI rloop get 10KB (c128) | 1249304 | 124909 | 1.021ms | 56.193ms |
| ASGI rloop echo 10KB (iter) (c128) | 616976 | 61688 | 2.067ms | 67.253ms |
| ASGI uvloop get 10KB (c128) | 1279888 | 127962 | 0.997ms | 34.37ms |
| ASGI uvloop echo 10KB (iter) (c128) | 621551 | 62146 | 2.053ms | 48.604ms |
| RSGI asyncio get 10KB (c128) | 834524 | 83449 | 1.527ms | 69.925ms |
| RSGI asyncio echo 10KB (iter) (c128) | 294914 | 29494 | 4.31ms | 55.038ms |
| RSGI rloop get 10KB (c128) | 1255607 | 125535 | 1.016ms | 41.315ms |
| RSGI rloop echo 10KB (iter) (c128) | 541029 | 54100 | 2.358ms | 50.119ms |
| RSGI uvloop get 10KB (c128) | 1253244 | 125285 | 1.017ms | 64.307ms |
| RSGI uvloop echo 10KB (iter) (c128) | 540926 | 54069 | 2.358ms | 64.165ms |

### Task implementation

Comparison between Granian Rust AsyncIO task implementation and stdlib one on ASGI protocol.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.10 | asyncio get 10KB (c128) | 746970 | 74686 | 1.707ms | 57.866ms |
| 3.10 | asyncio echo 10KB (iter) (c128) | 286332 | 28638 | 4.456ms | 56.752ms |
| 3.10 | rust get 10KB (c128) | 887365 | 88726 | 1.439ms | 24.96ms |
| 3.10 | rust echo 10KB (iter) (c128) | 264384 | 26444 | 4.822ms | 245.077ms |
| 3.11 | asyncio get 10KB (c128) | 784057 | 78399 | 1.626ms | 56.582ms |
| 3.11 | asyncio echo 10KB (iter) (c128) | 307002 | 30703 | 4.158ms | 61.533ms |
| 3.11 | rust get 10KB (c128) | 952725 | 95253 | 1.339ms | 48.13ms |
| 3.11 | rust echo 10KB (iter) (c128) | 290548 | 29058 | 4.392ms | 243.4ms |
