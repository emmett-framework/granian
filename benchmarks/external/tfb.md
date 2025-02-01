# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Fri 24 Jan 2025, 06:51    
Run ID: 4bea847f-bb2e-45f9-a723-95cba45eec14 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=4bea847f-bb2e-45f9-a723-95cba45eec14))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Robyn | 442768 | 442768.0 |
| Uvicorn (httptools) | 1005094 | 1005094.0 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3951527 | 2.09 |
| Granian (WSGI) | 1891582 | 1.0 |
| uWSGI | 180945 | 0.1 |
| uWSGI + Nginx | 26500 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1145402 | 0.84 |
| Granian (ASGI) | 1123107 | 0.82 |
| Granian (RSGI) | 1368795 | 1.0 |
| Robyn | 333809 | 0.24 |
| Uvicorn (httptools) | 855791 | 0.63 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2344978 | 1.69 |
| Granian (WSGI) | 1390208 | 1.0 |
| uWSGI | 187693 | 0.14 |
| uWSGI + Nginx | 14636 | 0.01 |


