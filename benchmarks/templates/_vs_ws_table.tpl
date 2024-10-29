| Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- |
{{ for key, runs in _data.items(): }}
{{ max_c, run = get_max_concurrency_run(runs, "throughput", "sum") }}
{{ if run["throughput"]["sum"]: }}
| {{ =key }} (c{{ =max_c }}) | {{ =round(run["throughput"]["send"]) }} | {{ =round(run["throughput"]["recv"]) }} | {{ =round(run["throughput"]["sum"]) }} |
{{ pass }}
{{ pass }}
