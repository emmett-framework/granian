# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Mon 17 Feb 2025, 16:41    
Run ID: 79248506-7230-4e43-99c1-c76d9e370664 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=79248506-7230-4e43-99c1-c76d9e370664))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1352710 | 0.77 |
| Granian (RSGI) | 1759340 | 1.0 |
| Robyn | 445984 | 0.25 |
| Uvicorn (httptools) | 1002205 | 0.57 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 4012934 | 2.13 |
| Granian (WSGI) | 1884498 | 1.0 |
| uWSGI | 178965 | 0.09 |
| uWSGI + Nginx | 25274 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1162518 | 0.96 |
| Granian (ASGI) | 1007479 | 0.83 |
| Granian (RSGI) | 1207963 | 1.0 |
| Robyn | 333562 | 0.28 |
| Uvicorn (httptools) | 851271 | 0.7 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2366747 | 1.69 |
| Granian (WSGI) | 1399954 | 1.0 |
| uWSGI | 188155 | 0.13 |
| uWSGI + Nginx | 14562 | 0.01 |


