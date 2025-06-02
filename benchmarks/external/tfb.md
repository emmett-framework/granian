# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Wed 21 May 2025, 10:30    
Run ID: 6f31fb63-2fda-481c-89d1-aa403f28c6a3 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=6f31fb63-2fda-481c-89d1-aa403f28c6a3))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1319541 | 0.76 |
| Granian (RSGI) | 1728148 | 1.0 |
| Robyn | 445357 | 0.26 |
| Uvicorn (httptools) | 1006022 | 0.58 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 4037014 | 1.99 |
| Granian (WSGI) | 2030986 | 1.0 |
| uWSGI | 180181 | 0.09 |
| uWSGI + Nginx | 26714 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1163065 | 0.97 |
| Granian (ASGI) | 998208 | 0.83 |
| Granian (RSGI) | 1203105 | 1.0 |
| Robyn | 335021 | 0.28 |
| Uvicorn (httptools) | 857136 | 0.71 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2364915 | 1.67 |
| Granian (WSGI) | 1416545 | 1.0 |
| uWSGI | 187919 | 0.13 |
| uWSGI + Nginx | 16062 | 0.01 |


