# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sat 10 Aug 2024, 20:32    
Run ID: 60149e78-2a0b-4e75-8c47-de4c3facf61f ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=60149e78-2a0b-4e75-8c47-de4c3facf61f))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1162530 | 0.58 |
| Granian (RSGI) | 2004241 | 1.0 |
| Robyn | 764514 | 0.38 |
| Uvicorn (httptools) | 1039683 | 0.52 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5293060 | 6.0 |
| Granian (WSGI) | 881777 | 1.0 |
| uWSGI | 181756 | 0.21 |
| uWSGI + Nginx | 27990 | 0.03 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1234458 | 1.01 |
| Granian (ASGI) | 933236 | 0.77 |
| Granian (RSGI) | 1216700 | 1.0 |
| Robyn | 535672 | 0.44 |
| Uvicorn (httptools) | 866587 | 0.71 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2678239 | 3.35 |
| Granian (WSGI) | 798833 | 1.0 |
| uWSGI | 190358 | 0.24 |
| uWSGI + Nginx | 16000 | 0.02 |


