# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sat 17 Aug 2024, 03:55    
Run ID: 3fcf7387-010c-4381-a26c-a46ccac69dd2 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=3fcf7387-010c-4381-a26c-a46ccac69dd2))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1157822 | 0.57 |
| Granian (RSGI) | 2035502 | 1.0 |
| Robyn | 765989 | 0.38 |
| Uvicorn (httptools) | 1045922 | 0.51 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5324672 | 6.02 |
| Granian (WSGI) | 884205 | 1.0 |
| uWSGI | 180810 | 0.2 |
| uWSGI + Nginx | 37108 | 0.04 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1232586 | 1.01 |
| Granian (ASGI) | 923021 | 0.76 |
| Granian (RSGI) | 1217631 | 1.0 |
| Robyn | 538189 | 0.44 |
| Uvicorn (httptools) | 870082 | 0.71 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2700300 | 3.34 |
| Granian (WSGI) | 809400 | 1.0 |
| uWSGI | 189420 | 0.23 |
| uWSGI + Nginx | 16011 | 0.02 |


