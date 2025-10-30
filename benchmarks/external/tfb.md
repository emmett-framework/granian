# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Fri 03 Oct 2025, 22:14    
Run ID: cda95417-edbb-4da3-ac87-10431d28b020 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=cda95417-edbb-4da3-ac87-10431d28b020))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1339402 | 0.76 |
| Granian (RSGI) | 1761589 | 1.0 |
| Robyn | 460020 | 0.26 |
| Socketify (ASGI) | 1130780 | 0.64 |
| Uvicorn (httptools) | 1004373 | 0.57 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3988154 | 2.0 |
| Granian (WSGI) | 1995964 | 1.0 |
| Socketify (WSGI) | 1491291 | 0.75 |
| uWSGI | 179177 | 0.09 |
| uWSGI + Nginx | 24898 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1150233 | 0.98 |
| Granian (ASGI) | 974936 | 0.83 |
| Granian (RSGI) | 1178058 | 1.0 |
| Robyn | 342815 | 0.29 |
| Socketify (ASGI) | 745424 | 0.63 |
| Uvicorn (httptools) | 849262 | 0.72 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2306061 | 1.67 |
| Granian (WSGI) | 1383366 | 1.0 |
| Socketify (WSGI) | 917120 | 0.66 |
| uWSGI | 187548 | 0.14 |
| uWSGI + Nginx | 15844 | 0.01 |


