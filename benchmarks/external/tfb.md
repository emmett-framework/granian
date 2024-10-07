# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Mon 30 Sep 2024, 10:04    
Run ID: c4ce919a-f64f-41bc-b6b7-23d1c7105b22 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=c4ce919a-f64f-41bc-b6b7-23d1c7105b22))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1370268 | 0.68 |
| Granian (RSGI) | 2015306 | 1.0 |
| Robyn | 408619 | 0.2 |
| Uvicorn (httptools) | 1045136 | 0.52 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5266722 | 5.51 |
| Granian (WSGI) | 956556 | 1.0 |
| uWSGI | 179832 | 0.19 |
| uWSGI + Nginx | 29611 | 0.03 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1241731 | 1.03 |
| Granian (ASGI) | 1084113 | 0.9 |
| Granian (RSGI) | 1199933 | 1.0 |
| Robyn | 294030 | 0.25 |
| Uvicorn (httptools) | 876846 | 0.73 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2677134 | 3.02 |
| Granian (WSGI) | 885073 | 1.0 |
| uWSGI | 188155 | 0.21 |
| uWSGI + Nginx | 16264 | 0.02 |


