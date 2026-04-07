# Granian benchmarks



## AsyncIO-specific benchmarks

Run at: Tue 07 Apr 2026, 12:20    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.12.77 (CPUs: 16)    
Granian version: 2.7.3

Same methodology of the main benchmarks applies.

### Event loops

Comparison between different AsyncIO event loops on async protocols.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| ASGI asyncio get 10KB (c128) | 916141 | 91607 | 1.392ms | 60.019ms |
| ASGI asyncio echo 10KB (iter) (c128) | 324790 | 32480 | 3.93ms | 49.462ms |
| ASGI rloop get 10KB (c128) | 1265080 | 126478 | 1.006ms | 92.91ms |
| ASGI rloop echo 10KB (iter) (c128) | 599838 | 59977 | 2.121ms | 111.599ms |
| ASGI uvloop get 10KB (c128) | 1276294 | 127606 | 0.999ms | 66.676ms |
| ASGI uvloop echo 10KB (iter) (c128) | 580238 | 58017 | 2.196ms | 73.357ms |
| RSGI asyncio get 10KB (c128) | 917394 | 91717 | 1.39ms | 56.99ms |
| RSGI asyncio echo 10KB (iter) (c128) | 327962 | 32798 | 3.877ms | 104.665ms |
| RSGI rloop get 10KB (c128) | 1250090 | 124962 | 1.02ms | 63.182ms |
| RSGI rloop echo 10KB (iter) (c128) | 541253 | 54121 | 2.357ms | 67.289ms |
| RSGI uvloop get 10KB (c128) | 1267614 | 126725 | 1.006ms | 45.78ms |
| RSGI uvloop echo 10KB (iter) (c128) | 541945 | 54190 | 2.355ms | 55.857ms |

### Task implementation

Comparison between Granian Rust AsyncIO task implementation and stdlib one on ASGI protocol.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.10 | asyncio get 10KB (c128) | 750823 | 75076 | 1.699ms | 49.305ms |
| 3.10 | asyncio echo 10KB (iter) (c128) | 279598 | 27963 | 4.561ms | 66.24ms |
| 3.10 | rust get 10KB (c128) | 908628 | 90848 | 1.404ms | 50.247ms |
| 3.10 | rust echo 10KB (iter) (c128) | 251689 | 25174 | 5.069ms | 314.779ms |
| 3.11 | asyncio get 10KB (c128) | 892728 | 89262 | 1.426ms | 86.476ms |
| 3.11 | asyncio echo 10KB (iter) (c128) | 306329 | 30635 | 4.166ms | 57.031ms |
| 3.11 | rust get 10KB (c128) | 1006687 | 100650 | 1.267ms | 50.449ms |
| 3.11 | rust echo 10KB (iter) (c128) | 269469 | 26950 | 4.735ms | 304.001ms |
