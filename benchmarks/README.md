# Granian benchmarks - Python 3.10

Run at: 2022-11-09T01:27:51.869362

CPUs: 2

## RSGI response types

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c128) | 812932 | 54049 | 2.183ms | 20.384ms |
| str small (c128) | 803649 | 53401 | 2.174ms | 14.245ms |
| bytes big (c32) | 12584 | 838 | 38.093ms | 65.667ms |
| str big (c32) | 267112 | 17775 | 1.232ms | 10.202ms |

## RSGI vs ASGI

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c128) | 826313 | 54963 | 2.132ms | 24.256ms |
| RSGI str (c128) | 805577 | 53545 | 2.203ms | 18.912ms |
| ASGI bytes (c128) | 474955 | 31537 | 3.96ms | 15.536ms |
| ASGI str (c128) | 484555 | 32165 | 3.803ms | 41.308ms |

## vs Uvicorn

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian ASGI (c128) | 439560 | 29204 | 4.308ms | 22.363ms |
| Granian RSGI (c128) | 833932 | 55467 | 2.115ms | 13.694ms |
| Uvicorn H11 (c16) | 148660 | 9905 | 1.615ms | 14.236ms |
| Uvicorn http-tools (c128) | 584789 | 38850 | 3.286ms | 28.537ms |

## Concurrency

### ASGI

| Concurrency | Threading mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| none | runtime (c32) | 168460 | 11227 | 2.847ms | 6.768ms |
| none | workers (c16) | 180505 | 12028 | 1.32ms | 4.416ms |
| min | runtime (c32) | 136032 | 9065 | 3.526ms | 12.583ms |
| min | workers (c16) | 142243 | 9420 | 1.694ms | 5.347ms |
| realistic | runtime (c128) | 476179 | 31624 | 3.939ms | 15.74ms |
| realistic | workers (c128) | 450457 | 29914 | 4.205ms | 19.397ms |
| max | runtime (c128) | 416600 | 27695 | 4.552ms | 24.319ms |
| max | workers (c128) | 415983 | 27625 | 4.591ms | 28.379ms |

### RSGI

| Concurrency | Threading mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| none | runtime (c64) | 373888 | 24883 | 2.548ms | 8.544ms |
| none | workers (c64) | 381359 | 25385 | 2.496ms | 6.413ms |
| min | runtime (c64) | 304100 | 20246 | 3.147ms | 15.214ms |
| min | workers (c128) | 342463 | 22758 | 5.639ms | 18.286ms |
| realistic | runtime (c128) | 821819 | 54621 | 2.139ms | 33.537ms |
| realistic | workers (c128) | 721706 | 47960 | 2.52ms | 22.3ms |
| max | runtime (c128) | 748466 | 49753 | 2.361ms | 24.37ms |
| max | workers (c128) | 720815 | 47937 | 2.522ms | 27.617ms |

# Granian benchmarks - Python 3.11

Run at: 2022-11-09T01:27:35.710499

CPUs: 2

## RSGI response types

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c64) | 525027 | 34959 | 1.823ms | 8.276ms |
| str small (c128) | 581517 | 38672 | 3.297ms | 12.681ms |
| bytes big (c64) | 15009 | 998 | 63.823ms | 85.737ms |
| str big (c128) | 258165 | 17104 | 5.676ms | 31.923ms |

## RSGI vs ASGI

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c128) | 509683 | 33873 | 3.826ms | 23.305ms |
| RSGI str (c32) | 512582 | 34170 | 0.939ms | 8.144ms |
| ASGI bytes (c128) | 333654 | 22176 | 5.761ms | 23.112ms |
| ASGI str (c64) | 335448 | 22336 | 2.865ms | 26.038ms |

## vs Uvicorn

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian ASGI (c64) | 346440 | 23066 | 2.779ms | 24.163ms |
| Granian RSGI (c128) | 455032 | 30260 | 4.219ms | 14.363ms |
| Uvicorn H11 (c128) | 219714 | 14591 | 8.732ms | 20.813ms |
| Uvicorn http-tools (c64) | 700307 | 46633 | 1.38ms | 8.183ms |

## Concurrency

### ASGI

| Concurrency | Threading mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| none | runtime (c32) | 115272 | 7680 | 4.162ms | 10.933ms |
| none | workers (c16) | 118275 | 7882 | 2.022ms | 14.365ms |
| min | runtime (c64) | 100959 | 6725 | 9.504ms | 18.199ms |
| min | workers (c128) | 122362 | 8133 | 15.688ms | 40.469ms |
| realistic | runtime (c32) | 100999 | 6731 | 4.75ms | 11.795ms |
| realistic | workers (c64) | 349791 | 23291 | 2.741ms | 9.979ms |
| max | runtime (c64) | 394604 | 26259 | 2.361ms | 8.448ms |
| max | workers (c128) | 343308 | 22831 | 5.601ms | 28.596ms |

### RSGI

| Concurrency | Threading mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| none | runtime (c128) | 230387 | 15312 | 8.313ms | 16.313ms |
| none | workers (c128) | 228188 | 15170 | 8.393ms | 17.245ms |
| min | runtime (c128) | 205628 | 13666 | 9.321ms | 21.606ms |
| min | workers (c64) | 208522 | 13877 | 4.658ms | 16.708ms |
| realistic | runtime (c128) | 544050 | 36162 | 3.556ms | 17.122ms |
| realistic | workers (c128) | 522582 | 34722 | 4.092ms | 30.384ms |
| max | runtime (c128) | 636904 | 42309 | 2.96ms | 19.028ms |
| max | workers (c128) | 552767 | 36765 | 3.956ms | 22.82ms |
