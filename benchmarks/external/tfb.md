# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Fri 23 Aug 2024, 11:54    
Run ID: ecd61f41-e3b3-42ef-8bcd-137aaa60dba5 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=ecd61f41-e3b3-42ef-8bcd-137aaa60dba5))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1148704 | 0.57 |
| Granian (RSGI) | 2009980 | 1.0 |
| Robyn | 765723 | 0.38 |
| Uvicorn (httptools) | 1040521 | 0.52 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5374920 | 6.09 |
| Granian (WSGI) | 882784 | 1.0 |
| uWSGI | 178781 | 0.2 |
| uWSGI + Nginx | 30058 | 0.03 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1233173 | 1.01 |
| Granian (ASGI) | 915276 | 0.75 |
| Granian (RSGI) | 1224172 | 1.0 |
| Robyn | 540768 | 0.44 |
| Uvicorn (httptools) | 870630 | 0.71 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2676951 | 3.33 |
| Granian (WSGI) | 803379 | 1.0 |
| uWSGI | 187286 | 0.23 |
| uWSGI + Nginx | 14544 | 0.02 |


