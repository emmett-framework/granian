# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Thu 30 Jan 2025, 18:47    
Run ID: 91a66052-9d86-446c-b31a-eadbd669ed08 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=91a66052-9d86-446c-b31a-eadbd669ed08))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1368373 | 0.78 |
| Granian (RSGI) | 1750813 | 1.0 |
| Robyn | 443426 | 0.25 |
| Uvicorn (httptools) | 1007683 | 0.58 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3983568 | 2.11 |
| Granian (WSGI) | 1888693 | 1.0 |
| uWSGI | 179975 | 0.1 |
| uWSGI + Nginx | 25290 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1158902 | 0.96 |
| Granian (ASGI) | 1017093 | 0.84 |
| Granian (RSGI) | 1204769 | 1.0 |
| Robyn | 334468 | 0.28 |
| Uvicorn (httptools) | 853263 | 0.71 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2370256 | 1.69 |
| Granian (WSGI) | 1402000 | 1.0 |
| uWSGI | 188583 | 0.13 |
| uWSGI + Nginx | 14819 | 0.01 |


