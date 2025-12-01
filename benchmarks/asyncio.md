# Granian benchmarks



## AsyncIO-specific benchmarks

Run at: Mon 01 Dec 2025, 17:06    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.12.58 (CPUs: 16)    
Granian version: 2.6.0

Same methodology of the main benchmarks applies.

### Event loops

Comparison between different AsyncIO event loops on async protocols.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| ASGI asyncio get 10KB (c128) | 798947 | 79887 | 1.597ms | 50.779ms |
| ASGI asyncio echo 10KB (iter) (c128) | 282059 | 28209 | 4.52ms | 76.216ms |
| ASGI rloop get 10KB (c128) | 1126880 | 112668 | 1.132ms | 63.735ms |
| ASGI rloop echo 10KB (iter) (c128) | 512066 | 51202 | 2.492ms | 64.075ms |
| ASGI uvloop get 10KB (c128) | 1127010 | 112673 | 1.131ms | 67.682ms |
| ASGI uvloop echo 10KB (iter) (c128) | 491635 | 49161 | 2.596ms | 58.79ms |
| RSGI asyncio get 10KB (c128) | 698196 | 69809 | 1.826ms | 74.661ms |
| RSGI asyncio echo 10KB (iter) (c128) | 272222 | 27228 | 4.684ms | 78.821ms |
| RSGI rloop get 10KB (c128) | 1256230 | 125585 | 1.014ms | 64.507ms |
| RSGI rloop echo 10KB (iter) (c128) | 547389 | 54734 | 2.326ms | 79.813ms |
| RSGI uvloop get 10KB (c128) | 1273363 | 127284 | 1.0ms | 77.455ms |
| RSGI uvloop echo 10KB (iter) (c128) | 539826 | 53980 | 2.362ms | 74.647ms |

### Task implementation

Comparison between Granian Rust AsyncIO task implementation and stdlib one on ASGI protocol.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.10 | asyncio get 10KB (c128) | 779907 | 77983 | 1.634ms | 68.703ms |
| 3.10 | asyncio echo 10KB (iter) (c128) | 292140 | 29217 | 4.369ms | 53.618ms |
| 3.10 | rust get 10KB (c128) | 962715 | 96251 | 1.324ms | 70.611ms |
| 3.10 | rust echo 10KB (iter) (c128) | 266362 | 26639 | 4.793ms | 302.092ms |
| 3.11 | asyncio get 10KB (c128) | 910211 | 91004 | 1.401ms | 47.926ms |
| 3.11 | asyncio echo 10KB (iter) (c128) | 315655 | 31568 | 4.04ms | 67.683ms |
| 3.11 | rust get 10KB (c128) | 1038961 | 103879 | 1.226ms | 76.857ms |
| 3.11 | rust echo 10KB (iter) (c128) | 281381 | 28142 | 4.535ms | 288.862ms |
