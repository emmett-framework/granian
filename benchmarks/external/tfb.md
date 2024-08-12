# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sun 04 Aug 2024, 12:26    
Run ID: b0a928a6-ff24-462e-8445-fa2dc1bbc7ee ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=b0a928a6-ff24-462e-8445-fa2dc1bbc7ee))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1166131 | 0.58 |
| Granian (RSGI) | 2017786 | 1.0 |
| Robyn | 769536 | 0.38 |
| Uvicorn (httptools) | 1044356 | 0.52 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5282034 | 6.0 |
| Granian (WSGI) | 879942 | 1.0 |
| uWSGI | 181006 | 0.21 |
| uWSGI + Nginx | 27557 | 0.03 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1244090 | 1.03 |
| Granian (ASGI) | 927179 | 0.76 |
| Granian (RSGI) | 1213523 | 1.0 |
| Robyn | 543473 | 0.45 |
| Uvicorn (httptools) | 861796 | 0.71 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2654621 | 3.3 |
| Granian (WSGI) | 805147 | 1.0 |
| uWSGI | 189019 | 0.23 |
| uWSGI + Nginx | 15303 | 0.02 |


