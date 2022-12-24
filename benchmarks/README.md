# Granian benchmarks

Run at: 2022-12-24T16:18:45.098896

CPUs: 2

## RSGI response types

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c128) | 542771 | 36078 | 3.537ms | 9.631ms |
| str small (c64) | 570614 | 37969 | 1.688ms | 7.12ms |
| bytes big (c64) | 6793 | 452 | 140.675ms | 148.736ms |
| str big (c32) | 245041 | 16330 | 1.954ms | 13.606ms |

## Interfaces

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c128) | 552696 | 36735 | 3.485ms | 14.695ms |
| RSGI str (c128) | 549006 | 36471 | 3.5ms | 13.68ms |
| ASGI bytes (c64) | 356831 | 23727 | 2.715ms | 16.297ms |
| ASGI str (c128) | 340393 | 22619 | 5.647ms | 18.226ms |
| WSGI bytes (c32) | 496912 | 33098 | 0.968ms | 12.507ms |
| WSGI str (c32) | 597875 | 39594 | 0.809ms | 10.341ms |

## vs 3rd parties

### async

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian ASGI (c32) | 340934 | 22723 | 1.405ms | 7.187ms |
| Granian RSGI (c128) | 504322 | 33520 | 3.806ms | 16.204ms |
| Uvicorn H11 (c64) | 86607 | 5763 | 11.032ms | 18.021ms |
| Uvicorn http-tools (c64) | 398311 | 26513 | 2.359ms | 7.234ms |
| Hypercorn (c32) | 21244 | 1415 | 22.514ms | 50.035ms |

### sync

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian WSGI (c32) | 358628 | 23901 | 1.361ms | 12.425ms |
| Gunicorn meinheld (c128) | 377223 | 25060 | 5.089ms | 31.482ms |



## Concurrency

### ASGI

| Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| P1 T1 wth (c32) | 369833 | 24646 | 1.296ms | 9.805ms |
| P1 T1 rth (c64) | 348848 | 23232 | 2.853ms | 61.176ms |
| P1 T2 wth (c128) | 364937 | 24239 | 5.267ms | 68.508ms |
| P1 T2 rth (c128) | 364872 | 24262 | 5.262ms | 16.738ms |
| P2 T1 wth (c64) | 485318 | 32286 | 2.018ms | 13.556ms |
| P2 T1 rth (c32) | 456040 | 30394 | 1.051ms | 5.655ms |
| P2 T2 wth (c128) | 551048 | 36664 | 3.488ms | 21.416ms |
| P2 T2 rth (c128) | 669873 | 44474 | 2.82ms | 12.25ms |

### RSGI

| Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| P1 T1 wth (c128) | 660383 | 43916 | 2.908ms | 11.842ms |
| P1 T1 rth (c64) | 402756 | 26803 | 2.369ms | 18.191ms |
| P1 T2 wth (c128) | 555691 | 36944 | 3.57ms | 23.035ms |
| P1 T2 rth (c64) | 402589 | 26804 | 2.38ms | 11.169ms |
| P2 T1 wth (c32) | 494607 | 32754 | 0.976ms | 5.41ms |
| P2 T1 rth (c64) | 651498 | 43365 | 1.478ms | 15.609ms |
| P2 T2 wth (c128) | 630891 | 41936 | 3.084ms | 32.546ms |
| P2 T2 rth (c128) | 860261 | 57152 | 2.06ms | 15.287ms |

### WSGI

| Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| P1 T1 wth (c64) | 671457 | 44707 | 1.429ms | 12.205ms |
| P1 T1 rth (c64) | 564375 | 37542 | 1.707ms | 10.247ms |
| P1 T2 wth (c64) | 712701 | 47460 | 1.364ms | 8.053ms |
| P1 T2 rth (c128) | 448673 | 29840 | 4.275ms | 11.408ms |
| P2 T1 wth (c16) | 638735 | 42300 | 0.385ms | 18.074ms |
| P2 T1 rth (c16) | 724302 | 47968 | 0.364ms | 10.494ms |
| P2 T2 wth (c32) | 706388 | 47090 | 0.682ms | 8.813ms |
| P2 T2 rth (c128) | 817544 | 54188 | 2.654ms | 23.652ms |
