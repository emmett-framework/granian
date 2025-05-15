# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sat 03 May 2025, 06:35    
Run ID: 6e3ad2d2-4acf-49cd-902a-286531ff3a67 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=6e3ad2d2-4acf-49cd-902a-286531ff3a67))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1335250 | 0.75 |
| Granian (RSGI) | 1775477 | 1.0 |
| Robyn | 443544 | 0.25 |
| Uvicorn (httptools) | 1005288 | 0.57 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3983258 | 1.95 |
| Granian (WSGI) | 2038894 | 1.0 |
| uWSGI | 180758 | 0.09 |
| uWSGI + Nginx | 26856 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1148556 | 0.97 |
| Granian (ASGI) | 994103 | 0.84 |
| Granian (RSGI) | 1189244 | 1.0 |
| Robyn | 333482 | 0.28 |
| Uvicorn (httptools) | 851680 | 0.72 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2349965 | 1.65 |
| Granian (WSGI) | 1422470 | 1.0 |
| uWSGI | 189013 | 0.13 |
| uWSGI + Nginx | 15937 | 0.01 |


