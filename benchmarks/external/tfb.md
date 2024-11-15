# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Thu 07 Nov 2024, 08:34    
Run ID: e81c1103-95d8-485e-949a-5ae323c76c87 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=e81c1103-95d8-485e-949a-5ae323c76c87))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1425043 | 0.67 |
| Granian (RSGI) | 2116979 | 1.0 |
| Robyn | 448593 | 0.21 |
| Uvicorn (httptools) | 1067429 | 0.5 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5313019 | 5.44 |
| Granian (WSGI) | 976117 | 1.0 |
| uWSGI | 178864 | 0.18 |
| uWSGI + Nginx | 29087 | 0.03 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1253745 | 1.0 |
| Granian (ASGI) | 1138750 | 0.91 |
| Granian (RSGI) | 1251065 | 1.0 |
| Robyn | 349430 | 0.28 |
| Uvicorn (httptools) | 898312 | 0.72 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2621901 | 2.88 |
| Granian (WSGI) | 911196 | 1.0 |
| uWSGI | 188248 | 0.21 |
| uWSGI + Nginx | 14832 | 0.02 |


