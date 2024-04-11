{{ include './_helpers.tpl' }}
# Granian benchmarks

## Concurrency

Run at: {{ =datetime.datetime.fromtimestamp(data.run_at).strftime('%a %d %b %Y, %H:%M') }}
Environment: {{ =benv }} (CPUs: {{ =data.cpu }})
Python version: {{ =data.pyver }}
Granian version: {{ =data.granian }}

{{ for interface in ["asgi", "rsgi", "wsgi"]: }}
### {{ =interface.upper() }}
{{ max_rps = {"runtime": 0, "workers": 0} }}
{{ for runs in data.results["concurrencies"][interface].values(): }}
{{ for crun in runs["res"].values(): }}
{{ max_rps[runs["m"]] = max(int(crun["requests"]["rps"]), max_rps[runs["m"]]) }}
{{ pass }}
{{ pass }}

| Mode | Processes | Threads | Blocking Threads | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- | --- |
{{ for runs in data.results["concurrencies"][interface].values(): }}
{{ max_c, run = get_max_concurrency_run(runs["res"]) }}
{{ if int(run["requests"]["rps"]) == max_rps[runs["m"]]: }}
| **{{ =runs["m"] }} (c{{ =max_c }})** | **{{ =runs["p"] }}** | **{{ =runs["t"] }}** | **{{ =runs["b"] }}** | **{{ =run["requests"]["total"] }}** | **{{ =run["requests"]["rps"] }}** | **{{ =fmt_ms(run["latency"]["avg"]) }}** | **{{ =fmt_ms(run["latency"]["max"]) }}** |
{{ else: }}
| {{ =runs["m"] }} (c{{ =max_c }}) | {{ =runs["p"] }} | {{ =runs["t"] }} | {{ =runs["b"] }} | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =fmt_ms(run["latency"]["avg"]) }} | {{ =fmt_ms(run["latency"]["max"]) }} |
{{ pass }}
{{ pass }}

{{ pass }}
