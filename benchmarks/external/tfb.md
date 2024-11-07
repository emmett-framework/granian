# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Fri 25 Oct 2024, 15:45    
Run ID: 1300aee6-f9c9-42a2-8d17-252ba597202f ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=1300aee6-f9c9-42a2-8d17-252ba597202f))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1419298 | 0.67 |
| Granian (RSGI) | 2103753 | 1.0 |
| Robyn | 446802 | 0.21 |
| Uvicorn (httptools) | 1060714 | 0.5 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5222009 | 5.39 |
| Granian (WSGI) | 968833 | 1.0 |
| uWSGI | 179458 | 0.19 |
| uWSGI + Nginx | 35017 | 0.04 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1259367 | 1.01 |
| Granian (ASGI) | 1134211 | 0.91 |
| Granian (RSGI) | 1249365 | 1.0 |
| Robyn | 346778 | 0.28 |
| Uvicorn (httptools) | 890883 | 0.71 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2712297 | 3.01 |
| Granian (WSGI) | 901408 | 1.0 |
| uWSGI | 188934 | 0.21 |
| uWSGI + Nginx | 16074 | 0.02 |


