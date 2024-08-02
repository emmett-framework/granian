# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Tue 23 Jul 2024, 02:53    
Run ID: df867f5b-9adc-48fb-ba7d-601062c16c42 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=df867f5b-9adc-48fb-ba7d-601062c16c42))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1154450 | 0.57 |
| Granian (RSGI) | 2028565 | 1.0 |
| Robyn | 767892 | 0.38 |
| Uvicorn (httptools) | 1037550 | 0.51 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5320411 | 6.04 |
| Granian (WSGI) | 880249 | 1.0 |
| uWSGI | 180324 | 0.2 |
| uWSGI + Nginx | 28075 | 0.03 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1237459 | 1.01 |
| Granian (ASGI) | 924366 | 0.76 |
| Granian (RSGI) | 1223478 | 1.0 |
| Robyn | 539965 | 0.44 |
| Uvicorn (httptools) | 874261 | 0.71 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2653182 | 3.3 |
| Granian (WSGI) | 804229 | 1.0 |
| uWSGI | 190263 | 0.24 |
| uWSGI + Nginx | 14662 | 0.02 |


