# Granian benchmarks - Python 3.10

Run at: 2022-10-31T17:12:08.876192

CPUs: 2

## RSGI response types

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c128) | 607251 | 40331 | 2.946ms | 25.687ms |
| str small (c128) | 615617 | 40866 | 2.89ms | 25.915ms |
| bytes big (c16) | 10516 | 700 | 22.823ms | 51.192ms |
| str big (c32) | 196851 | 13073 | 1.801ms | 18.045ms |

## RSGI vs ASGI

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c128) | 621939 | 41222 | 2.902ms | 22.212ms |
| RSGI str (c128) | 694060 | 45975 | 2.56ms | 35.966ms |
| ASGI bytes (c128) | 317740 | 21071 | 5.948ms | 27.614ms |
| ASGI str (c128) | 331795 | 22005 | 5.663ms | 27.255ms |

## vs Uvicorn

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian ASGI (c128) | 332302 | 22048 | 5.585ms | 24.444ms |
| Granian RSGI (c128) | 603387 | 40056 | 3.001ms | 21.871ms |
| Uvicorn H11 (c128) | 104432 | 6934 | 18.385ms | 80.237ms |
| Uvicorn http-tools (c128) | 410821 | 27260 | 4.686ms | 30.736ms |

## Concurrency

### ASGI

| Concurrency | Threading mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| none | runtime (c64) | 89417 | 5952 | 10.748ms | 30.215ms |
| none | workers (c32) | 103271 | 6877 | 4.671ms | 22.133ms |
| min | runtime (c128) | 82400 | 5480 | 23.281ms | 45.508ms |
| min | workers (c128) | 110182 | 7326 | 17.481ms | 58.597ms |
| realistic | runtime (c16) | 77282 | 5150 | 3.143ms | 20.066ms |
| realistic | workers (c16) | 140685 | 9376 | 1.742ms | 18.569ms |
| max | runtime (c128) | 310694 | 20611 | 6.092ms | 33.809ms |
| max | workers (c128) | 315316 | 20942 | 6.127ms | 37.006ms |

### RSGI

| Concurrency | Threading mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| none | runtime (c128) | 208580 | 13852 | 9.192ms | 35.284ms |
| none | workers (c128) | 229219 | 15223 | 8.433ms | 60.556ms |
| min | runtime (c128) | 187869 | 12485 | 10.211ms | 43.581ms |
| min | workers (c128) | 229005 | 15209 | 8.499ms | 50.836ms |
| realistic | runtime (c128) | 610766 | 40520 | 2.892ms | 26.11ms |
| realistic | workers (c128) | 530515 | 35233 | 3.73ms | 32.572ms |
| max | runtime (c128) | 513733 | 34122 | 3.476ms | 26.035ms |
| max | workers (c128) | 502648 | 33386 | 3.664ms | 33.941ms |

# Granian benchmarks - Python 3.11

Run at: 2022-10-31T17:09:47.249453

CPUs: 2

## RSGI response types

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c64) | 481809 | 32083 | 1.999ms | 9.336ms |
| str small (c64) | 479483 | 31915 | 2.001ms | 11.107ms |
| bytes big (c128) | 14061 | 933 | 135.717ms | 235.983ms |
| str big (c32) | 243120 | 16162 | 1.349ms | 9.947ms |

## RSGI vs ASGI

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c128) | 484015 | 32178 | 3.979ms | 18.08ms |
| RSGI str (c128) | 488460 | 32492 | 3.936ms | 16.565ms |
| ASGI bytes (c64) | 313847 | 20896 | 3.058ms | 10.834ms |
| ASGI str (c64) | 320907 | 21368 | 2.996ms | 28.127ms |

## vs Uvicorn

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian ASGI (c128) | 322667 | 21456 | 5.949ms | 20.031ms |
| Granian RSGI (c64) | 488526 | 32524 | 2.0ms | 10.884ms |
| Uvicorn H11 (c128) | 194693 | 12935 | 9.853ms | 33.233ms |
| Uvicorn http-tools (c128) | 707563 | 47019 | 2.699ms | 9.49ms |

## Concurrency

### ASGI

| Concurrency | Threading mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| none | runtime (c32) | 110799 | 7382 | 4.33ms | 10.224ms |
| none | workers (c16) | 115343 | 7686 | 2.07ms | 4.708ms |
| min | runtime (c32) | 97805 | 6518 | 4.905ms | 9.92ms |
| min | workers (c32) | 116642 | 7772 | 4.143ms | 14.014ms |
| realistic | runtime (c64) | 315085 | 20977 | 3.042ms | 9.924ms |
| realistic | workers (c128) | 339120 | 22494 | 5.629ms | 29.884ms |
| max | runtime (c128) | 346920 | 23007 | 5.458ms | 23.548ms |
| max | workers (c64) | 295168 | 19661 | 3.274ms | 26.226ms |

### RSGI

| Concurrency | Threading mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
| none | runtime (c128) | 239457 | 15897 | 8.002ms | 14.425ms |
| none | workers (c64) | 228192 | 15192 | 4.188ms | 8.49ms |
| min | runtime (c128) | 220338 | 14649 | 8.7ms | 20.487ms |
| min | workers (c128) | 220463 | 14648 | 8.699ms | 22.568ms |
| realistic | runtime (c128) | 473667 | 31501 | 4.051ms | 14.133ms |
| realistic | workers (c128) | 475150 | 31579 | 4.292ms | 44.868ms |
| max | runtime (c128) | 599842 | 39845 | 3.12ms | 16.55ms |
| max | workers (c128) | 556072 | 36885 | 4.019ms | 24.546ms |
