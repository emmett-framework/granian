# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Thu 29 Aug 2024, 19:56    
Run ID: 9e73c44c-3e85-4888-8697-f0014dc38a2b ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=9e73c44c-3e85-4888-8697-f0014dc38a2b))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1163890 | 0.58 |
| Granian (RSGI) | 2020313 | 1.0 |
| Robyn | 413300 | 0.2 |
| Uvicorn (httptools) | 1042639 | 0.52 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5363726 | 6.06 |
| Granian (WSGI) | 884386 | 1.0 |
| uWSGI | 180074 | 0.2 |
| uWSGI + Nginx | 28316 | 0.03 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1240562 | 1.02 |
| Granian (ASGI) | 916314 | 0.76 |
| Granian (RSGI) | 1213231 | 1.0 |
| Robyn | 295073 | 0.24 |
| Uvicorn (httptools) | 873377 | 0.72 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2666539 | 3.3 |
| Granian (WSGI) | 807591 | 1.0 |
| uWSGI | 188773 | 0.23 |
| uWSGI + Nginx | 12810 | 0.02 |


