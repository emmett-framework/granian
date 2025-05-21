# Granian benchmarks



Run at: Tue 20 May 2025, 23:59    
Environment: AMD Ryzen 7 5700X @ Ubuntu 24.04 (CPUs: 16)    
Python version: 3.12    
Granian version: 2.3.1

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes 10B (c128) | 1320835 | 132259 | 0.966ms | 26.702ms |
| str 10B (c64) | 1302873 | 130294 | 0.49ms | 1.767ms |
| bytes 100KB (c64) | 622644 | 62268 | 1.026ms | 4.943ms |
| str 100KB (c64) | 605439 | 60546 | 1.055ms | 4.895ms |


## Interfaces

Comparison between Granian application protocols using plain text responses.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI get 1KB (c128) | 1291114 | 129244 | 0.989ms | 24.037ms |
| RSGI echo 1KB (c128) | 1248460 | 124963 | 1.021ms | 25.555ms |
| RSGI echo 100KB (iter) (c64) | 224463 | 22447 | 2.846ms | 9.111ms |
| ASGI get 1KB (c256) | 1287544 | 128971 | 1.983ms | 21.859ms |
| ASGI echo 1KB (c128) | 930149 | 93129 | 1.37ms | 19.798ms |
| ASGI echo 100KB (iter) (c64) | 198128 | 19812 | 3.226ms | 10.243ms |
| WSGI get 1KB (c64) | 1306352 | 130645 | 0.489ms | 1.67ms |
| WSGI echo 1KB (c64) | 1263731 | 126378 | 0.506ms | 1.688ms |
| WSGI echo 100KB (iter) (c64) | 91339 | 9135 | 6.989ms | 22.023ms |


## HTTP/2

Comparison between Granian HTTP versions on RSGI using plain text responses.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| HTTP/1 get 1KB (c256) | 1293291 | 129832 | 1.969ms | 22.481ms |
| HTTP/1 echo 1KB (c256) | 1237148 | 123896 | 2.06ms | 37.443ms |
| HTTP/2 get 1KB (c128) | 742247 | 74327 | 1.719ms | 27.693ms |
| HTTP/2 echo 1KB (c512) | 123518 | 12416 | 41.037ms | 114.657ms |


## File responses

Comparison between Granian application protocols using ~50KB image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI (c256) | 512562 | 51392 | 4.976ms | 34.612ms |
| ASGI (c128) | 305898 | 30615 | 4.175ms | 25.425ms |
| ASGI pathsend (c128) | 494641 | 49514 | 2.581ms | 27.304ms |


### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [AsyncIO-specific benchmarks](./asyncio.md)
- [Python versions](./pyver.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
