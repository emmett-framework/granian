# Granian benchmarks

Run at: 2023-01-13T00:27:29.008349

CPUs: 2

## RSGI response types

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c128) | 570943 | 37941 | 3.359ms | 13.477ms |
| str small (c128) | 682598 | 45397 | 2.826ms | 16.29ms |
| bytes big (c64) | 7342 | 488 | 130.32ms | 134.452ms |
| str big (c16) | 299082 | 19926 | 0.798ms | 13.497ms |

## Interfaces

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c128) | 588253 | 39128 | 3.267ms | 16.773ms |
| RSGI str (c128) | 584231 | 38861 | 3.278ms | 9.84ms |
| ASGI bytes (c128) | 404399 | 26888 | 4.744ms | 13.287ms |
| ASGI str (c128) | 426655 | 28391 | 4.495ms | 13.05ms |
| WSGI bytes (c64) | 707850 | 47113 | 1.354ms | 8.759ms |
| WSGI str (c32) | 641393 | 42753 | 0.748ms | 8.173ms |

## vs 3rd parties

### async

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian ASGI (c128) | 291503 | 19377 | 6.601ms | 24.624ms |
| Granian RSGI (c128) | 371631 | 24702 | 5.246ms | 22.019ms |
| Uvicorn H11 (c64) | 59932 | 3988 | 15.974ms | 34.812ms |
| Uvicorn http-tools (c128) | 279559 | 18576 | 6.802ms | 21.073ms |
| Hypercorn (c16) | 14944 | 994 | 16.014ms | 50.812ms |

### sync

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian WSGI (c64) | 708664 | 47188 | 1.352ms | 5.842ms |
| Gunicorn meinheld (c128) | 702808 | 46702 | 2.735ms | 21.696ms |



## Concurrency

### ASGI

| Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| P1 T1 wth (c128) | 366243 | 24342 | 5.248ms | 16.911ms |
| P1 T1 rth (c128) | 283054 | 18818 | 6.775ms | 15.366ms |
| P1 T2 wth (c32) | 311514 | 20760 | 1.543ms | 9.3ms |
| P1 T2 rth (c128) | 282808 | 18806 | 6.795ms | 23.744ms |
| P2 T1 wth (c128) | 355342 | 23632 | 5.45ms | 19.885ms |
| P2 T1 rth (c128) | 378459 | 25167 | 5.096ms | 18.943ms |
| P2 T2 wth (c32) | 408744 | 27246 | 1.201ms | 25.984ms |
| P2 T2 rth (c128) | 498309 | 33121 | 3.889ms | 22.309ms |

### RSGI

| Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| P1 T1 wth (c128) | 451250 | 30009 | 4.25ms | 10.984ms |
| P1 T1 rth (c16) | 343301 | 22886 | 0.701ms | 6.998ms |
| P1 T2 wth (c64) | 537239 | 35768 | 1.815ms | 25.291ms |
| P1 T2 rth (c128) | 375974 | 25007 | 5.094ms | 18.34ms |
| P2 T1 wth (c128) | 408164 | 27147 | 4.7ms | 11.497ms |
| P2 T1 rth (c128) | 514905 | 34224 | 3.738ms | 24.623ms |
| P2 T2 wth (c64) | 611090 | 40601 | 1.388ms | 24.457ms |
| P2 T2 rth (c128) | 788191 | 52280 | 1.987ms | 19.913ms |

### WSGI

| Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| P1 T1 wth (c64) | 498237 | 33165 | 1.923ms | 5.556ms |
| P1 T1 rth (c64) | 453163 | 30146 | 2.119ms | 10.188ms |
| P1 T2 wth (c128) | 553592 | 36815 | 3.476ms | 18.021ms |
| P1 T2 rth (c16) | 313307 | 20885 | 0.767ms | 8.873ms |
| P2 T1 wth (c32) | 499637 | 33088 | 0.97ms | 10.607ms |
| P2 T1 rth (c16) | 558307 | 36974 | 0.441ms | 12.831ms |
| P2 T2 wth (c64) | 524710 | 34865 | 1.603ms | 19.914ms |
| P2 T2 rth (c128) | 573477 | 38041 | 3.578ms | 24.08ms |

