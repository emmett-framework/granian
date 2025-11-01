# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sun 12 Oct 2025, 10:44    
Run ID: 26b8a037-cc59-44d5-85d9-edad4876aa6f ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=26b8a037-cc59-44d5-85d9-edad4876aa6f))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1343768 | 0.76 |
| Granian (RSGI) | 1764333 | 1.0 |
| Robyn | 462487 | 0.26 |
| Socketify (ASGI) | 1125447 | 0.64 |
| Uvicorn (httptools) | 999812 | 0.57 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3969116 | 1.98 |
| Granian (WSGI) | 2005337 | 1.0 |
| Socketify (WSGI) | 1494163 | 0.75 |
| uWSGI | 178160 | 0.09 |
| uWSGI + Nginx | 25098 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1140017 | 0.97 |
| Granian (ASGI) | 980505 | 0.83 |
| Granian (RSGI) | 1177753 | 1.0 |
| Robyn | 342325 | 0.29 |
| Socketify (ASGI) | 740134 | 0.63 |
| Uvicorn (httptools) | 855920 | 0.73 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2319616 | 1.68 |
| Granian (WSGI) | 1379428 | 1.0 |
| Socketify (WSGI) | 919819 | 0.67 |
| uWSGI | 185963 | 0.13 |
| uWSGI + Nginx | 12672 | 0.01 |


