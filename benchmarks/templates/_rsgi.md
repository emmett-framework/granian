## RSGI response types

RSGI plain text response comparison using protocol `response_str` and `response_bytes`.    
The "small" response is 4 bytes, the "big" one is 80kbytes.

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ for key, runs in _data.items(): }}
{{ max_c, run = get_max_concurrency_run(runs) }}
| {{ =key }} (c{{ =max_c }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =fmt_ms(run["latency"]["avg"]) }} | {{ =fmt_ms(run["latency"]["max"]) }} |
{{ pass }}
