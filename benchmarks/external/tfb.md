# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Thu 05 Sep 2024, 04:03    
Run ID: 50404e8f-0e56-424c-ad18-add6337e3720 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=50404e8f-0e56-424c-ad18-add6337e3720))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1151471 | 0.57 |
| Granian (RSGI) | 2033998 | 1.0 |
| Robyn | 411879 | 0.2 |
| Uvicorn (httptools) | 1043039 | 0.51 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5312938 | 5.98 |
| Granian (WSGI) | 888260 | 1.0 |
| uWSGI | 179904 | 0.2 |
| uWSGI + Nginx | 41576 | 0.05 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1235038 | 1.02 |
| Granian (ASGI) | 909801 | 0.75 |
| Granian (RSGI) | 1214392 | 1.0 |
| Robyn | 295455 | 0.24 |
| Uvicorn (httptools) | 876373 | 0.72 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2707239 | 3.37 |
| Granian (WSGI) | 802872 | 1.0 |
| uWSGI | 190147 | 0.24 |
| uWSGI + Nginx | 16394 | 0.02 |


