{{ import datetime }}
{{ import multiprocessing }}
# Granian benchmarks

Run at: {{ =data.run_at }}

CPUs: {{ =data.cpu }}

## RSGI response types

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ for key, runs in data.results["rsgi_body"].items(): }}
{{ concurrency_values = {runs[ckey]["requests"]["rps"]: ckey for ckey in runs.keys()} }}
{{ max_res = concurrency_values[max(concurrency_values.keys())] }}
{{ run = runs[max_res] }}
| {{ =key }} (c{{ =max_res }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =int(run["latency"]["avg"]) / 1000 }}ms | {{ =int(run["latency"]["max"]) / 1000 }}ms |
{{ pass }}

## RSGI vs ASGI

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ for key, runs in data.results["rsgi_asgi"].items(): }}
{{ concurrency_values = {runs[ckey]["requests"]["rps"]: ckey for ckey in runs.keys()} }}
{{ max_res = concurrency_values[max(concurrency_values.keys())] }}
{{ run = runs[max_res] }}
| {{ =key }} (c{{ =max_res }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =int(run["latency"]["avg"]) / 1000 }}ms | {{ =int(run["latency"]["max"]) / 1000 }}ms |
{{ pass }}

## vs Uvicorn

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ for key, runs in data.results["uvicorn"].items(): }}
{{ concurrency_values = {runs[ckey]["requests"]["rps"]: ckey for ckey in runs.keys()} }}
{{ max_res = concurrency_values[max(concurrency_values.keys())] }}
{{ run = runs[max_res] }}
| {{ =key }} (c{{ =max_res }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =int(run["latency"]["avg"]) / 1000 }}ms | {{ =int(run["latency"]["max"]) / 1000 }}ms |
{{ pass }}

{{ if "procs_threads" in data.results: }}
## Threading

{{ for interface in ["asgi", "rsgi"]: }}
### {{ =interface.upper() }}

| Threading mode | Procs | Threads | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- |
{{ for key in data.results["procs_threads"][interface]["runtime"].keys(): }}
{{ for thmode in ["runtime", "workers"]: }}
{{ runs = data.results["procs_threads"][interface][thmode][key] }}
{{ concurrency_values = {runs[ckey]["requests"]["rps"]: ckey for ckey in runs.keys()} }}
{{ max_res = concurrency_values[max(concurrency_values.keys())] }}
{{ run = runs[max_res] }}
| {{ =thmode }} (c{{ =max_res }}) | {{ =key.split(" ")[0] }} | {{ =key.split(" ")[3] }} | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =int(run["latency"]["avg"]) / 1000 }}ms | {{ =int(run["latency"]["max"]) / 1000 }}ms |
{{ pass }}
{{ pass }}

{{ pass }}
{{ pass }}
