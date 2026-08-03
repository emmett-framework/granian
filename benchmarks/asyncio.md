# Granian benchmarks



## AsyncIO-specific benchmarks

Run at: Mon 03 Aug 2026, 13:33    
Environment: AMD Ryzen 7 5700X @ Gentoo Linux 6.18.41 (CPUs: 16)    
Granian version: 2.8.0

Same methodology of the main benchmarks applies.

### Event loops

Comparison between different AsyncIO event loops on async protocols.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| ASGI asyncio get 10KB (c128) | 839230 | 83908 | 1.52ms | 51.656ms |
| ASGI asyncio echo 10KB (iter) (c128) | 310612 | 31068 | 4.108ms | 55.179ms |
| ASGI rloop get 10KB (c128) | 1215411 | 121514 | 1.05ms | 35.207ms |
| ASGI rloop echo 10KB (iter) (c128) | 543235 | 54320 | 2.348ms | 68.411ms |
| ASGI uvloop get 10KB (c128) | 1208549 | 120829 | 1.055ms | 43.209ms |
| ASGI uvloop echo 10KB (iter) (c128) | 543266 | 54322 | 2.349ms | 71.63ms |
| RSGI asyncio get 10KB (c128) | 912162 | 91202 | 1.399ms | 42.773ms |
| RSGI asyncio echo 10KB (iter) (c128) | 335161 | 33518 | 3.808ms | 50.721ms |
| RSGI rloop get 10KB (c128) | 1250666 | 125034 | 1.019ms | 74.128ms |
| RSGI rloop echo 10KB (iter) (c128) | 541749 | 54166 | 2.356ms | 60.262ms |
| RSGI uvloop get 10KB (c128) | 1247640 | 124725 | 1.022ms | 50.592ms |
| RSGI uvloop echo 10KB (iter) (c128) | 542087 | 54206 | 2.351ms | 64.009ms |

### Task implementation

Comparison between Granian Rust AsyncIO task implementation and stdlib one on ASGI protocol.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| 3.10 | asyncio get 10KB (c128) | 767313 | 76721 | 1.664ms | 36.178ms |
| 3.10 | asyncio echo 10KB (iter) (c128) | 271892 | 27196 | 4.693ms | 61.188ms |
| 3.10 | rust get 10KB (c128) | 910217 | 91018 | 1.399ms | 87.943ms |
| 3.10 | rust echo 10KB (iter) (c128) | 247852 | 24789 | 5.149ms | 243.031ms |
| 3.11 | asyncio get 10KB (c128) | 786990 | 78685 | 1.62ms | 49.041ms |
| 3.11 | asyncio echo 10KB (iter) (c128) | 296161 | 29621 | 4.308ms | 53.257ms |
| 3.11 | rust get 10KB (c128) | 909852 | 90966 | 1.401ms | 58.006ms |
| 3.11 | rust echo 10KB (iter) (c128) | 281718 | 28177 | 4.533ms | 230.648ms |
