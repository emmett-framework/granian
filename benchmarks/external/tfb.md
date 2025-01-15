# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sat 04 Jan 2025, 20:22    
Run ID: 924216cc-3d66-43a2-ad8b-4d423e968e3e ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=924216cc-3d66-43a2-ad8b-4d423e968e3e))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Robyn | 444864 | 444864.0 |
| Uvicorn (httptools) | 1006557 | 1006557.0 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 4003751 | 2.13 |
| Granian (WSGI) | 1882079 | 1.0 |
| uWSGI | 181088 | 0.1 |
| uWSGI + Nginx | 26659 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1156226 | 0.85 |
| Granian (ASGI) | 1106764 | 0.81 |
| Granian (RSGI) | 1359305 | 1.0 |
| Robyn | 332771 | 0.24 |
| Uvicorn (httptools) | 849258 | 0.62 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2387502 | 1.71 |
| Granian (WSGI) | 1396252 | 1.0 |
| uWSGI | 189776 | 0.14 |
| uWSGI + Nginx | 15513 | 0.01 |


