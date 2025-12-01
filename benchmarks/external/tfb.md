# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Mon 24 Nov 2025, 17:55    
Run ID: 3be5b7f5-ca41-451e-af3a-a0e7213e24cb ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=3be5b7f5-ca41-451e-af3a-a0e7213e24cb))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1293750 | 0.76 |
| Granian (RSGI) | 1709328 | 1.0 |
| Robyn | 460778 | 0.27 |
| Socketify (ASGI) | 1123937 | 0.66 |
| Uvicorn (httptools) | 1012232 | 0.59 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3800650 | 1.97 |
| Granian (WSGI) | 1928562 | 1.0 |
| Socketify (WSGI) | 1508947 | 0.78 |
| uWSGI | 177353 | 0.09 |
| uWSGI + Nginx | 24756 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1150995 | 0.98 |
| Granian (ASGI) | 958922 | 0.82 |
| Granian (RSGI) | 1168712 | 1.0 |
| Robyn | 347916 | 0.3 |
| Socketify (ASGI) | 740605 | 0.63 |
| Uvicorn (httptools) | 850209 | 0.73 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2289844 | 1.69 |
| Granian (WSGI) | 1352116 | 1.0 |
| Socketify (WSGI) | 921468 | 0.68 |
| uWSGI | 184499 | 0.14 |
| uWSGI + Nginx | 9967 | 0.01 |


