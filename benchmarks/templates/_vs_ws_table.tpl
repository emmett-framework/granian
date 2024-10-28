| Server | Send throughput | Receive throughput | Throughput (total) |
| --- | --- | --- | --- |
{{ for key, runs in _data.items(): }}
{{ max_c, run = get_max_concurrency_run(runs, "throughput", "sum") }}
| {{ =key }} (c{{ =max_c }}) | {{ =round(run["throughput"]["send"]) }} | {{ =round(run["throughput"]["recv"]) }} | {{ =round(run["throughput"]["sum"]) }} |
{{ pass }}
{{ pass }}
