# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Wed 17 Dec 2025, 00:04    
Run ID: 3b45c2ae-0f9f-438c-9f2a-3034fb7271ce ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=3b45c2ae-0f9f-438c-9f2a-3034fb7271ce))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1303611 | 0.75 |
| Granian (RSGI) | 1731499 | 1.0 |
| Robyn | 462610 | 0.27 |
| Socketify (ASGI) | 1125456 | 0.65 |
| Uvicorn (httptools) | 1006975 | 0.58 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3960890 | 2.04 |
| Granian (WSGI) | 1945412 | 1.0 |
| Socketify (WSGI) | 1505740 | 0.77 |
| uWSGI | 177999 | 0.09 |
| uWSGI + Nginx | 25059 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1154477 | 0.99 |
| Granian (ASGI) | 959642 | 0.82 |
| Granian (RSGI) | 1169126 | 1.0 |
| Robyn | 343523 | 0.29 |
| Socketify (ASGI) | 742228 | 0.63 |
| Uvicorn (httptools) | 849112 | 0.73 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2297814 | 1.69 |
| Granian (WSGI) | 1356365 | 1.0 |
| Socketify (WSGI) | 919007 | 0.68 |
| uWSGI | 185433 | 0.14 |
| uWSGI + Nginx | 12462 | 0.01 |


