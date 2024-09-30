# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Tue 17 Sep 2024, 20:06    
Run ID: c35fdca1-35ea-4411-b75a-5766e7833e59 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=c35fdca1-35ea-4411-b75a-5766e7833e59))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1334204 | 0.66 |
| Granian (RSGI) | 2027985 | 1.0 |
| Robyn | 410295 | 0.2 |
| Uvicorn (httptools) | 1046041 | 0.52 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5327593 | 5.55 |
| Granian (WSGI) | 960564 | 1.0 |
| uWSGI | 180024 | 0.19 |
| uWSGI + Nginx | 28115 | 0.03 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1231267 | 1.02 |
| Granian (ASGI) | 1048943 | 0.87 |
| Granian (RSGI) | 1206261 | 1.0 |
| Robyn | 294835 | 0.24 |
| Uvicorn (httptools) | 872496 | 0.72 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2641129 | 2.96 |
| Granian (WSGI) | 892816 | 1.0 |
| uWSGI | 188501 | 0.21 |
| uWSGI + Nginx | 15549 | 0.02 |


