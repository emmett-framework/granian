| Server | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ for key, runs in _data.items(): }}
{{ max_c, run = get_max_concurrency_run(runs) }}
| {{ =key }} (c{{ =max_c }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =fmt_ms(run["latency"]["avg"]) }} | {{ =fmt_ms(run["latency"]["max"]) }} |
{{ pass }}
{{ pass }}
