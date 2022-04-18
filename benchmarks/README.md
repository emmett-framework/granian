# Granian benchmarks

Run at: 2022-04-18T16:43:31.401254

Workers: 2

## RSGI response types

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| bytes small (c128) | 618207 | 41057 | 3.053ms | 20.421ms |
| str small (c128) | 616316 | 40922 | 3.082ms | 31.887ms |
| bytes big (c64) | 12047 | 801 | 79.528ms | 117.872ms |
| str big (c128) | 262742 | 17462 | 6.591ms | 38.733ms |

## RSGI vs ASGI

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| RSGI bytes (c128) | 631201 | 41874 | 2.99ms | 27.755ms |
| RSGI str (c128) | 668312 | 44370 | 2.813ms | 27.598ms |
| ASGI bytes (c128) | 460246 | 30541 | 4.099ms | 29.323ms |
| ASGI str (c128) | 449328 | 29776 | 4.203ms | 23.635ms |

## vs Uvicorn

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
| Granian ASGI (c128) | 463698 | 30732 | 4.07ms | 32.791ms |
| Granian RSGI (c128) | 633132 | 42088 | 2.981ms | 18.95ms |
| Uvicorn H11 (c32) | 122764 | 8178 | 3.908ms | 11.433ms |
| Uvicorn http-tools (c128) | 464464 | 30885 | 4.136ms | 16.718ms |
