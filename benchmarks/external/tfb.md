# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sun 02 Mar 2025, 18:53    
Run ID: 3ed2adc4-4636-42e7-8894-983c04a62c1b ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=3ed2adc4-4636-42e7-8894-983c04a62c1b))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1362259 | 0.77 |
| Granian (RSGI) | 1759305 | 1.0 |
| Robyn | 443935 | 0.25 |
| Uvicorn (httptools) | 1005511 | 0.57 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 4035957 | 2.16 |
| Granian (WSGI) | 1872136 | 1.0 |
| uWSGI | 180213 | 0.1 |
| uWSGI + Nginx | 26940 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1166556 | 0.96 |
| Granian (ASGI) | 1018853 | 0.84 |
| Granian (RSGI) | 1214461 | 1.0 |
| Robyn | 335754 | 0.28 |
| Uvicorn (httptools) | 850203 | 0.7 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2347010 | 1.67 |
| Granian (WSGI) | 1402353 | 1.0 |
| uWSGI | 188396 | 0.13 |
| uWSGI + Nginx | 14150 | 0.01 |


