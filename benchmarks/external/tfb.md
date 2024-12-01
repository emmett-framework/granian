# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Wed 13 Nov 2024, 15:41    
Run ID: fd4f1f27-72cd-4e89-92c6-0d965fadb733 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=fd4f1f27-72cd-4e89-92c6-0d965fadb733))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1412296 | 0.66 |
| Granian (RSGI) | 2127360 | 1.0 |
| Robyn | 450131 | 0.21 |
| Uvicorn (httptools) | 1076743 | 0.51 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5332306 | 5.5 |
| Granian (WSGI) | 969970 | 1.0 |
| uWSGI | 179879 | 0.19 |
| uWSGI + Nginx | 27627 | 0.03 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1257736 | 1.0 |
| Granian (ASGI) | 1127747 | 0.89 |
| Granian (RSGI) | 1260891 | 1.0 |
| Robyn | 351114 | 0.28 |
| Uvicorn (httptools) | 896046 | 0.71 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2684726 | 2.95 |
| Granian (WSGI) | 910953 | 1.0 |
| uWSGI | 188389 | 0.21 |
| uWSGI + Nginx | 16355 | 0.02 |


