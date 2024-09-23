# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Wed 11 Sep 2024, 11:49    
Run ID: 235f2c13-5e49-44b8-9dc1-1d087d6353bc ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=235f2c13-5e49-44b8-9dc1-1d087d6353bc))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1340546 | 0.67 |
| Granian (RSGI) | 2014776 | 1.0 |
| Robyn | 409213 | 0.2 |
| Uvicorn (httptools) | 1043862 | 0.52 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5279410 | 5.5 |
| Granian (WSGI) | 959940 | 1.0 |
| uWSGI | 179795 | 0.19 |
| uWSGI + Nginx | 34411 | 0.04 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1231557 | 1.02 |
| Granian (ASGI) | 1060216 | 0.87 |
| Granian (RSGI) | 1212887 | 1.0 |
| Robyn | 293436 | 0.24 |
| Uvicorn (httptools) | 880072 | 0.73 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2653663 | 2.99 |
| Granian (WSGI) | 887557 | 1.0 |
| uWSGI | 186963 | 0.21 |
| uWSGI + Nginx | 16392 | 0.02 |


