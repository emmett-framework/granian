# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Sun 18 Jan 2026, 21:13    
Run ID: acc1ad82-ae2c-4d1d-a600-c7ff9d0c5917 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=acc1ad82-ae2c-4d1d-a600-c7ff9d0c5917))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1307677 | 0.76 |
| Granian (RSGI) | 1729099 | 1.0 |
| Robyn | 462385 | 0.27 |
| Socketify (ASGI) | 1113204 | 0.64 |
| Uvicorn (httptools) | 996806 | 0.58 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3915232 | 2.0 |
| Granian (WSGI) | 1956488 | 1.0 |
| Socketify (WSGI) | 1491981 | 0.76 |
| uWSGI | 179472 | 0.09 |
| uWSGI + Nginx | 24738 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1150506 | 0.99 |
| Granian (ASGI) | 963069 | 0.83 |
| Granian (RSGI) | 1167153 | 1.0 |
| Robyn | 340870 | 0.29 |
| Socketify (ASGI) | 733533 | 0.63 |
| Uvicorn (httptools) | 844382 | 0.72 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2257646 | 1.67 |
| Granian (WSGI) | 1355427 | 1.0 |
| Socketify (WSGI) | 921158 | 0.68 |
| uWSGI | 185622 | 0.14 |
| uWSGI + Nginx | 11240 | 0.01 |


