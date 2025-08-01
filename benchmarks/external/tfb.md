# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sun 20 Jul 2025, 00:14    
Run ID: b4d86ad8-764f-47a1-a9b1-83c71bd8b6e9 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=b4d86ad8-764f-47a1-a9b1-83c71bd8b6e9))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1322219 | 0.75 |
| Granian (RSGI) | 1770733 | 1.0 |
| Robyn | 445532 | 0.25 |
| Socketify (ASGI) | 1265827 | 0.71 |
| Uvicorn (httptools) | 997473 | 0.56 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 4001185 | 1.99 |
| Granian (WSGI) | 2011428 | 1.0 |
| Socketify (WSGI) | 1760017 | 0.88 |
| uWSGI | 179955 | 0.09 |
| uWSGI + Nginx | 27734 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1179590 | 0.99 |
| Granian (ASGI) | 996654 | 0.83 |
| Granian (RSGI) | 1193973 | 1.0 |
| Robyn | 334171 | 0.28 |
| Socketify (ASGI) | 814202 | 0.68 |
| Uvicorn (httptools) | 859681 | 0.72 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2383317 | 1.67 |
| Granian (WSGI) | 1425324 | 1.0 |
| Socketify (WSGI) | 1029806 | 0.72 |
| uWSGI | 188054 | 0.13 |
| uWSGI + Nginx | 16130 | 0.01 |


