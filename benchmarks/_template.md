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

## Interfaces

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ for key, runs in data.results["interfaces"].items(): }}
{{ concurrency_values = {runs[ckey]["requests"]["rps"]: ckey for ckey in runs.keys()} }}
{{ max_res = concurrency_values[max(concurrency_values.keys())] }}
{{ run = runs[max_res] }}
| {{ =key }} (c{{ =max_res }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =int(run["latency"]["avg"]) / 1000 }}ms | {{ =int(run["latency"]["max"]) / 1000 }}ms |
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

| Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ for key, runs in data.results["concurrencies"][interface].items(): }}
{{ concurrency_values = {runs[ckey]["requests"]["rps"]: ckey for ckey in runs.keys()} }}
{{ max_res = concurrency_values[max(concurrency_values.keys())] }}
{{ run = runs[max_res] }}
| {{ =key }} (c{{ =max_res }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =int(run["latency"]["avg"]) / 1000 }}ms | {{ =int(run["latency"]["max"]) / 1000 }}ms |
{{ pass }}

{{ pass }}
{{ pass }}
