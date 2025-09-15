# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Tue 02 Sep 2025, 14:53    
Run ID: 3ab00ae1-17aa-44e6-ae83-137d797d0817 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=3ab00ae1-17aa-44e6-ae83-137d797d0817))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1341499 | 0.76 |
| Granian (RSGI) | 1762457 | 1.0 |
| Robyn | 461394 | 0.26 |
| Socketify (ASGI) | 1269210 | 0.72 |
| Uvicorn (httptools) | 1001961 | 0.57 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3891060 | 1.96 |
| Granian (WSGI) | 1989663 | 1.0 |
| Socketify (WSGI) | 1716946 | 0.86 |
| uWSGI | 177164 | 0.09 |
| uWSGI + Nginx | 25641 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1152822 | 0.98 |
| Granian (ASGI) | 979748 | 0.83 |
| Granian (RSGI) | 1175031 | 1.0 |
| Robyn | 343213 | 0.29 |
| Socketify (ASGI) | 813325 | 0.69 |
| Uvicorn (httptools) | 857101 | 0.73 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2309950 | 1.68 |
| Granian (WSGI) | 1375551 | 1.0 |
| Socketify (WSGI) | 1011324 | 0.74 |
| uWSGI | 185106 | 0.13 |
| uWSGI + Nginx | 9977 | 0.01 |


