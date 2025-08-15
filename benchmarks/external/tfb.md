# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sat 02 Aug 2025, 10:25    
Run ID: 809d8655-c602-42a1-9d8c-dc4692738790 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=809d8655-c602-42a1-9d8c-dc4692738790))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1325365 | 0.76 |
| Granian (RSGI) | 1749316 | 1.0 |
| Robyn | 446084 | 0.26 |
| Socketify (ASGI) | 1265694 | 0.72 |
| Uvicorn (httptools) | 1006909 | 0.58 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 4013405 | 1.98 |
| Granian (WSGI) | 2027688 | 1.0 |
| Socketify (WSGI) | 1708983 | 0.84 |
| uWSGI | 179066 | 0.09 |
| uWSGI + Nginx | 28209 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1154396 | 0.96 |
| Granian (ASGI) | 1000676 | 0.83 |
| Granian (RSGI) | 1199805 | 1.0 |
| Robyn | 334553 | 0.28 |
| Socketify (ASGI) | 819889 | 0.68 |
| Uvicorn (httptools) | 860341 | 0.72 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2374704 | 1.66 |
| Granian (WSGI) | 1428876 | 1.0 |
| Socketify (WSGI) | 1007744 | 0.71 |
| uWSGI | 187934 | 0.13 |
| uWSGI + Nginx | 16027 | 0.01 |


