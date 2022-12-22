# Granian benchmarks

Run at: 2022-12-22T15:40:40.441840

CPUs: 2

## RSGI response types

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c128) | 725390 | 48260 | 2.655ms | 12.012ms |
| str small (c64) | 618776 | 41192 | 1.547ms | 3.867ms |
| bytes big (c32) | 7262 | 483 | 65.998ms | 68.958ms |
| str big (c64) | 292313 | 19454 | 3.257ms | 6.707ms |

## Interfaces

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c128) | 668512 | 44469 | 2.875ms | 11.947ms |
| RSGI str (c128) | 583007 | 38772 | 3.292ms | 11.416ms |
| ASGI bytes (c64) | 222584 | 14820 | 4.307ms | 10.085ms |
| ASGI str (c16) | 226676 | 15107 | 1.053ms | 5.385ms |
| WSGI bytes (c16) | 725179 | 48026 | 0.33ms | 3.328ms |
| WSGI str (c64) | 729150 | 48553 | 1.313ms | 4.178ms |

## vs 3rd parties

### async

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian ASGI (c16) | 200308 | 13349 | 1.19ms | 5.85ms |
| Granian RSGI (c64) | 493291 | 32860 | 1.943ms | 7.803ms |
| Uvicorn H11 (c32) | 84582 | 5633 | 5.635ms | 11.395ms |
| Uvicorn http-tools (c64) | 391978 | 26088 | 2.396ms | 6.489ms |
| Hypercorn (c32) | 20009 | 1332 | 23.761ms | 54.669ms |

### sync

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian WSGI (c64) | 505795 | 33677 | 1.898ms | 11.425ms |
| Gunicorn meinheld (c128) | 565238 | 37591 | 3.393ms | 19.902ms |

### concurrency

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian ASGI (c128) | 519199 | 34505 | 3.609ms | 13.95ms |
| Granian RSGI (c128) | 625018 | 41582 | 3.127ms | 21.783ms |
| Granian WSGI (c128) | 627782 | 41751 | 3.054ms | 13.595ms |
| Uvicorn http-tools (c64) | 584417 | 38909 | 1.64ms | 9.175ms |
| Hypercorn (c128) | 23335 | 1550 | 82.805ms | 283.305ms |
| Gunicorn meinheld (c64) | 720059 | 47923 | 1.424ms | 17.058ms |

## Concurrency

### ASGI

| Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| P1 T1 (c16) | 203167 | 13540 | 1.175ms | 5.897ms |
| P1 T2 (c64) | 206359 | 13734 | 4.67ms | 21.886ms |
| P2 T1 (c128) | 474074 | 31485 | 4.0ms | 15.335ms |

### RSGI

| Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| P1 T1 (c128) | 687648 | 45730 | 2.808ms | 10.555ms |
| P1 T2 (c128) | 591438 | 39324 | 3.282ms | 20.661ms |
| P2 T1 (c64) | 467274 | 31108 | 2.086ms | 23.158ms |

### WSGI

| Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| P1 T1 (c16) | 700562 | 46396 | 0.343ms | 5.611ms |
| P1 T2 (c128) | 696358 | 46324 | 2.763ms | 17.47ms |
| P2 T1 (c32) | 633740 | 42228 | 0.763ms | 13.658ms |

