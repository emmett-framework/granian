# Granian benchmarks

Run at: {{ =data.run_at }}

CPUs: {{ =data.cpu }}
Python version: {{ =data.pyver }}

{{ if "rsgi_body" in data.results: }}
## RSGI response types

| Type | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ for key, runs in data.results["rsgi_body"].items(): }}
{{ concurrency_values = {runs[ckey]["requests"]["rps"]: ckey for ckey in runs.keys()} }}
{{ max_res = concurrency_values[max(concurrency_values.keys())] }}
{{ run = runs[max_res] }}
| {{ =key }} (c{{ =max_res }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =int(run["latency"]["avg"]) / 1000 }}ms | {{ =int(run["latency"]["max"]) / 1000 }}ms |
{{ pass }}

{{ pass }}

{{ if "interfaces" in data.results: }}
## Interfaces

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ for key, runs in data.results["interfaces"].items(): }}
{{ concurrency_values = {runs[ckey]["requests"]["rps"]: ckey for ckey in runs.keys()} }}
{{ max_res = concurrency_values[max(concurrency_values.keys())] }}
{{ run = runs[max_res] }}
| {{ =key }} (c{{ =max_res }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =int(run["latency"]["avg"]) / 1000 }}ms | {{ =int(run["latency"]["max"]) / 1000 }}ms |
{{ pass }}

{{ pass }}

{{ if any(key in data.results for key in ["vs_async", "vs_sync"]): }}
## vs 3rd parties

{{ if "vs_async" in data.results: }}
### async

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ for key, runs in data.results["vs_async"].items(): }}
{{ concurrency_values = {runs[ckey]["requests"]["rps"]: ckey for ckey in runs.keys()} }}
{{ max_res = concurrency_values[max(concurrency_values.keys())] }}
{{ run = runs[max_res] }}
| {{ =key }} (c{{ =max_res }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =int(run["latency"]["avg"]) / 1000 }}ms | {{ =int(run["latency"]["max"]) / 1000 }}ms |
{{ pass }}
{{ pass }}

{{ if "vs_sync" in data.results: }}
### sync

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ for key, runs in data.results["vs_sync"].items(): }}
{{ concurrency_values = {runs[ckey]["requests"]["rps"]: ckey for ckey in runs.keys()} }}
{{ max_res = concurrency_values[max(concurrency_values.keys())] }}
{{ run = runs[max_res] }}
| {{ =key }} (c{{ =max_res }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =int(run["latency"]["avg"]) / 1000 }}ms | {{ =int(run["latency"]["max"]) / 1000 }}ms |
{{ pass }}
{{ pass }}

{{ if "vs_maxc" in data.results: }}
### concurrency

| Mode | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ for key, runs in data.results["vs_maxc"].items(): }}
{{ concurrency_values = {runs[ckey]["requests"]["rps"]: ckey for ckey in runs.keys()} }}
{{ max_res = concurrency_values[max(concurrency_values.keys())] }}
{{ run = runs[max_res] }}
| {{ =key }} (c{{ =max_res }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =int(run["latency"]["avg"]) / 1000 }}ms | {{ =int(run["latency"]["max"]) / 1000 }}ms |
{{ pass }}
{{ pass }}

{{ pass }}

{{ if "concurrencies" in data.results: }}
## Concurrency

{{ for interface in ["asgi", "rsgi", "wsgi"]: }}
### {{ =interface.upper() }}
{{ max_rps = {"runtime": 0, "workers": 0} }}
{{ for runs in data.results["concurrencies"][interface].values(): }}
{{ for crun in runs["res"].values(): }}
{{ max_rps[runs["m"]] = max(crun["requests"]["rps"], max_rps[runs["m"]]) }}
{{ pass }}
{{ pass }}
{{ pass }}

| Mode | Processes | Threads | Blocking Threads | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- | --- |
{{ for runs in data.results["concurrencies"][interface].values(): }}
{{ concurrency_values = {runs["res"][ckey]["requests"]["rps"]: ckey for ckey in runs["res"].keys()} }}
{{ max_res = concurrency_values[max(concurrency_values.keys())] }}
{{ run = runs["res"][max_res] }}
{{ rps = "**" + run["requests"]["rps"] + "**" if run["requests"]["rps"] == max_rps[runs["m"]] else run["requests"]["rps"] }}
| {{ =runs["m"] }} (c{{ =max_res }}) | {{ =runs["p"] }} | {{ =runs["t"] }} | {{ =runs["b"] }} | {{ =run["requests"]["total"] }} | {{ =rps }} | {{ =int(run["latency"]["avg"]) / 1000 }}ms | {{ =int(run["latency"]["max"]) / 1000 }}ms |
{{ pass }}

{{ pass }}
{{ pass }}
