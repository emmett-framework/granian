# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Fri 02 Jan 2026, 14:00    
Run ID: f59a9c0c-19a4-4dab-b0f5-f8f175091bfa ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=f59a9c0c-19a4-4dab-b0f5-f8f175091bfa))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1310444 | 0.75 |
| Granian (RSGI) | 1738647 | 1.0 |
| Robyn | 464229 | 0.27 |
| Socketify (ASGI) | 1118611 | 0.64 |
| Uvicorn (httptools) | 1002396 | 0.58 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3908542 | 2.02 |
| Granian (WSGI) | 1938777 | 1.0 |
| Socketify (WSGI) | 1500888 | 0.77 |
| uWSGI | 178192 | 0.09 |
| uWSGI + Nginx | 24714 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1148709 | 0.99 |
| Granian (ASGI) | 962539 | 0.83 |
| Granian (RSGI) | 1162134 | 1.0 |
| Robyn | 341775 | 0.29 |
| Socketify (ASGI) | 737803 | 0.63 |
| Uvicorn (httptools) | 852882 | 0.73 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2269104 | 1.68 |
| Granian (WSGI) | 1349675 | 1.0 |
| Socketify (WSGI) | 920889 | 0.68 |
| uWSGI | 185172 | 0.14 |
| uWSGI + Nginx | 14181 | 0.01 |


