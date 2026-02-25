# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Wed 04 Feb 2026, 07:14    
Run ID: bf916986-96a5-4912-b2e7-45c2afa14165 ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=bf916986-96a5-4912-b2e7-45c2afa14165))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1307570 | 0.75 |
| Granian (RSGI) | 1736093 | 1.0 |
| Robyn | 463843 | 0.27 |
| Socketify (ASGI) | 1124990 | 0.65 |
| Uvicorn (httptools) | 993388 | 0.57 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3816669 | 1.95 |
| Granian (WSGI) | 1954866 | 1.0 |
| Socketify (WSGI) | 1490992 | 0.76 |
| uWSGI | 176880 | 0.09 |
| uWSGI + Nginx | 25255 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1145664 | 0.98 |
| Granian (ASGI) | 961078 | 0.82 |
| Granian (RSGI) | 1169086 | 1.0 |
| Robyn | 343610 | 0.29 |
| Socketify (ASGI) | 738307 | 0.63 |
| Uvicorn (httptools) | 844784 | 0.72 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2326750 | 1.72 |
| Granian (WSGI) | 1355412 | 1.0 |
| Socketify (WSGI) | 913641 | 0.67 |
| uWSGI | 184687 | 0.14 |
| uWSGI + Nginx | 13332 | 0.01 |


