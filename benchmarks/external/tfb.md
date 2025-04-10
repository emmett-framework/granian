# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sat 22 Mar 2025, 10:05    
Run ID: be04c3fa-c6fa-449b-a7ec-d099d4ba74ab ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=be04c3fa-c6fa-449b-a7ec-d099d4ba74ab))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1389259 | 0.79 |
| Granian (RSGI) | 1765056 | 1.0 |
| Robyn | 445955 | 0.25 |
| Uvicorn (httptools) | 1005624 | 0.57 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3940474 | 2.07 |
| Granian (WSGI) | 1899083 | 1.0 |
| uWSGI | 179020 | 0.09 |
| uWSGI + Nginx | 26018 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1144192 | 0.95 |
| Granian (ASGI) | 1025226 | 0.85 |
| Granian (RSGI) | 1210194 | 1.0 |
| Robyn | 335386 | 0.28 |
| Uvicorn (httptools) | 850612 | 0.7 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2375238 | 1.67 |
| Granian (WSGI) | 1418331 | 1.0 |
| uWSGI | 187033 | 0.13 |
| uWSGI + Nginx | 16133 | 0.01 |


