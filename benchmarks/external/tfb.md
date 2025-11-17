# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Tue 04 Nov 2025, 14:08    
Run ID: 40103b8c-8670-4734-8d39-7081cf9441d8 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=40103b8c-8670-4734-8d39-7081cf9441d8))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1295676 | 0.76 |
| Granian (RSGI) | 1711472 | 1.0 |
| Robyn | 462633 | 0.27 |
| Socketify (ASGI) | 1115389 | 0.65 |
| Uvicorn (httptools) | 999977 | 0.58 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3911493 | 2.01 |
| Granian (WSGI) | 1941632 | 1.0 |
| Socketify (WSGI) | 1496498 | 0.77 |
| uWSGI | 176791 | 0.09 |
| uWSGI + Nginx | 24704 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1160077 | 0.99 |
| Granian (ASGI) | 961613 | 0.82 |
| Granian (RSGI) | 1168796 | 1.0 |
| Robyn | 341486 | 0.29 |
| Socketify (ASGI) | 734394 | 0.63 |
| Uvicorn (httptools) | 843998 | 0.72 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2305388 | 1.7 |
| Granian (WSGI) | 1354243 | 1.0 |
| Socketify (WSGI) | 911639 | 0.67 |
| uWSGI | 184494 | 0.14 |
| uWSGI + Nginx | 12882 | 0.01 |


