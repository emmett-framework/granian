# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Mon 29 Jul 2024, 07:40    
Run ID: 96ee4baf-23c0-4ae1-8a53-04bb27333f97 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=96ee4baf-23c0-4ae1-8a53-04bb27333f97))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1153635 | 0.57 |
| Granian (RSGI) | 2020729 | 1.0 |
| Robyn | 764858 | 0.38 |
| Uvicorn (httptools) | 1043533 | 0.52 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5274281 | 6.03 |
| Granian (WSGI) | 874471 | 1.0 |
| uWSGI | 181155 | 0.21 |
| uWSGI + Nginx | 28507 | 0.03 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1231900 | 1.01 |
| Granian (ASGI) | 916142 | 0.75 |
| Granian (RSGI) | 1218345 | 1.0 |
| Robyn | 540500 | 0.44 |
| Uvicorn (httptools) | 868742 | 0.71 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2649915 | 3.34 |
| Granian (WSGI) | 793205 | 1.0 |
| uWSGI | 190336 | 0.24 |
| uWSGI + Nginx | 15263 | 0.02 |


