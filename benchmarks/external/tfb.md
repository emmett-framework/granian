# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sun 06 Oct 2024, 17:26    
Run ID: 176ba510-3607-4faa-996e-74f0778b88d4 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=176ba510-3607-4faa-996e-74f0778b88d4))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1371870 | 0.68 |
| Granian (RSGI) | 2014991 | 1.0 |
| Robyn | 408426 | 0.2 |
| Uvicorn (httptools) | 1049521 | 0.52 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 5292776 | 5.55 |
| Granian (WSGI) | 954325 | 1.0 |
| uWSGI | 178875 | 0.19 |
| uWSGI + Nginx | 27983 | 0.03 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1243555 | 1.04 |
| Granian (ASGI) | 1098131 | 0.91 |
| Granian (RSGI) | 1200801 | 1.0 |
| Robyn | 294450 | 0.25 |
| Uvicorn (httptools) | 884032 | 0.74 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2660843 | 2.99 |
| Granian (WSGI) | 889723 | 1.0 |
| uWSGI | 186718 | 0.21 |
| uWSGI + Nginx | 15332 | 0.02 |


