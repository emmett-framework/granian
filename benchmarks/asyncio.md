# Granian benchmarks



## AsyncIO-specific benchmarks

Run at: Wed 21 May 2025, 02:00    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Granian version: 2.3.1

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

### Event loops

Comparison between different AsyncIO event loops on async protocols.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| ASGI asyncio get 10KB (c256) | 846379 | 84868 | 3.014ms | 26.978ms |
| ASGI asyncio echo 10KB (iter) (c256) | 300980 | 30178 | 8.465ms | 30.471ms |
| ASGI rloop get 10KB (c128) | 1205713 | 120616 | 1.059ms | 11.959ms |
| ASGI rloop echo 10KB (iter) (c64) | 541032 | 54107 | 1.181ms | 2.702ms |
| ASGI uvloop get 10KB (c128) | 1191113 | 119178 | 1.072ms | 26.634ms |
| ASGI uvloop echo 10KB (iter) (c256) | 591319 | 59310 | 4.308ms | 33.738ms |
| RSGI asyncio get 10KB (c128) | 746271 | 74658 | 1.713ms | 15.645ms |
| RSGI asyncio echo 10KB (iter) (c128) | 276172 | 27641 | 4.622ms | 28.828ms |
| RSGI rloop get 10KB (c256) | 1199596 | 120299 | 2.126ms | 25.705ms |
| RSGI rloop echo 10KB (iter) (c256) | 668674 | 67025 | 3.812ms | 52.641ms |
| RSGI uvloop get 10KB (c64) | 1216601 | 121673 | 0.524ms | 3.124ms |
| RSGI uvloop echo 10KB (iter) (c256) | 776848 | 77899 | 3.282ms | 31.627ms |

### Task implementation

Comparison between Granian Rust AsyncIO task implementation and stdlib one on ASGI protocol.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.9 | asyncio get 10KB (c128) | 777437 | 77852 | 1.64ms | 29.001ms |
| 3.9 | asyncio echo 10KB (iter) (c256) | 289123 | 28991 | 8.817ms | 32.758ms |
| 3.9 | rust get 10KB (c128) | 1086159 | 108758 | 1.173ms | 24.085ms |
| 3.9 | rust echo 10KB (iter) (c256) | 262140 | 26276 | 9.724ms | 667.428ms |
| 3.10 | asyncio get 10KB (c256) | 815199 | 81745 | 3.127ms | 26.334ms |
| 3.10 | asyncio echo 10KB (iter) (c128) | 302098 | 30221 | 4.225ms | 30.409ms |
| 3.10 | rust get 10KB (c256) | 1124681 | 112789 | 2.261ms | 43.881ms |
| 3.10 | rust echo 10KB (iter) (c128) | 269919 | 27007 | 4.731ms | 431.828ms |
| 3.11 | asyncio get 10KB (c512) | 826252 | 83097 | 6.156ms | 77.654ms |
| 3.11 | asyncio echo 10KB (iter) (c128) | 308807 | 30896 | 4.138ms | 19.296ms |
| 3.11 | rust get 10KB (c512) | 1035518 | 104118 | 4.912ms | 97.311ms |
| 3.11 | rust echo 10KB (iter) (c256) | 282532 | 28329 | 9.016ms | 639.548ms |
