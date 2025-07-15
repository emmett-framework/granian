# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Tue 08 Jul 2025, 06:01    
Run ID: fbe03b07-c62b-45d0-9387-50c9429ea4fd ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=fbe03b07-c62b-45d0-9387-50c9429ea4fd))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1328892 | 0.75 |
| Granian (RSGI) | 1769600 | 1.0 |
| Robyn | 445228 | 0.25 |
| Socketify (ASGI) | 1273391 | 0.72 |
| Uvicorn (httptools) | 996445 | 0.56 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 4057048 | 1.98 |
| Granian (WSGI) | 2045247 | 1.0 |
| Socketify (WSGI) | 1741605 | 0.85 |
| uWSGI | 179254 | 0.09 |
| uWSGI + Nginx | 28126 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1156656 | 0.97 |
| Granian (ASGI) | 1004474 | 0.84 |
| Granian (RSGI) | 1192393 | 1.0 |
| Robyn | 334843 | 0.28 |
| Socketify (ASGI) | 815991 | 0.68 |
| Uvicorn (httptools) | 852237 | 0.71 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2344417 | 1.63 |
| Granian (WSGI) | 1434689 | 1.0 |
| Socketify (WSGI) | 1017699 | 0.71 |
| uWSGI | 187734 | 0.13 |
| uWSGI + Nginx | 13628 | 0.01 |


