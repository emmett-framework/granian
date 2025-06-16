# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sun 08 Jun 2025, 08:14    
Run ID: 416d5cf7-11ed-4ed7-8cf5-f3e476dc38b9 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=416d5cf7-11ed-4ed7-8cf5-f3e476dc38b9))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1322616 | 0.75 |
| Granian (RSGI) | 1772554 | 1.0 |
| Robyn | 446279 | 0.25 |
| Socketify (ASGI) | 1259852 | 0.71 |
| Uvicorn (httptools) | 994910 | 0.56 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 4012266 | 1.96 |
| Granian (WSGI) | 2044151 | 1.0 |
| Socketify (WSGI) | 1710893 | 0.84 |
| uWSGI | 180260 | 0.09 |
| uWSGI + Nginx | 26957 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1167713 | 0.97 |
| Granian (ASGI) | 995333 | 0.83 |
| Granian (RSGI) | 1199635 | 1.0 |
| Robyn | 334274 | 0.28 |
| Socketify (ASGI) | 814152 | 0.68 |
| Uvicorn (httptools) | 850518 | 0.71 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2370763 | 1.67 |
| Granian (WSGI) | 1422664 | 1.0 |
| Socketify (WSGI) | 1002493 | 0.7 |
| uWSGI | 188967 | 0.13 |
| uWSGI + Nginx | 15669 | 0.01 |


