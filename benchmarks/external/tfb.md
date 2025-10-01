# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: Thu 18 Sep 2025, 12:54    
Run ID: ae7d5979-b4f1-4494-897d-1eb9e0067cff ([visualize](https://www.techempower.com/benchmarks/#section=test&runid=ae7d5979-b4f1-4494-897d-1eb9e0067cff))


### Plain text


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Granian (ASGI) | 1340151 | 0.76 |
| Granian (RSGI) | 1757920 | 1.0 |
| Robyn | 460727 | 0.26 |
| Socketify (ASGI) | 1124721 | 0.64 |
| Uvicorn (httptools) | 1001938 | 0.57 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 3917897 | 1.96 |
| Granian (WSGI) | 1998433 | 1.0 |
| Socketify (WSGI) | 1493670 | 0.75 |
| uWSGI | 176826 | 0.09 |
| uWSGI + Nginx | 25369 | 0.01 |



### JSON


#### Async

| Server | RPS | Change (rate) |
| --- | --- | --- |
| FastWSGI (ASGI) | 1146237 | 0.97 |
| Granian (ASGI) | 978361 | 0.83 |
| Granian (RSGI) | 1178916 | 1.0 |
| Robyn | 340000 | 0.29 |
| Socketify (ASGI) | 741921 | 0.63 |
| Uvicorn (httptools) | 848552 | 0.72 |

#### Sync

| Server | RPS | Change (rate) |
| --- | --- | --- |
| Fastwsgi | 2275059 | 1.65 |
| Granian (WSGI) | 1380163 | 1.0 |
| Socketify (WSGI) | 917219 | 0.66 |
| uWSGI | 184307 | 0.13 |
| uWSGI + Nginx | 12989 | 0.01 |


