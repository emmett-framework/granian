# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Fri 20 Feb 2026, 14:50    
Run ID: e4388834-e02e-45e6-92ed-929bfe264a56 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=e4388834-e02e-45e6-92ed-929bfe264a56))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1313639 | 0.76 |
| Granian (RSGI) | 1730956 | 1.0 |
| Robyn | 464002 | 0.27 |
| Socketify (ASGI) | 1128456 | 0.65 |
| Uvicorn (httptools) | 992230 | 0.57 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3896110 | 2.0 |
| Granian (WSGI) | 1949070 | 1.0 |
| Socketify (WSGI) | 1491510 | 0.77 |
| uWSGI | 177996 | 0.09 |
| uWSGI + Nginx | 24958 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1154740 | 0.99 |
| Granian (ASGI) | 963306 | 0.83 |
| Granian (RSGI) | 1167461 | 1.0 |
| Robyn | 342769 | 0.29 |
| Socketify (ASGI) | 742341 | 0.64 |
| Uvicorn (httptools) | 849388 | 0.73 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2238940 | 1.65 |
| Granian (WSGI) | 1355248 | 1.0 |
| Socketify (WSGI) | 916673 | 0.68 |
| uWSGI | 185396 | 0.14 |
| uWSGI + Nginx | 12274 | 0.01 |


