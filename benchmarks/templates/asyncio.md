# Granian benchmarks

{{ include './_helpers.tpl' }}

## AsyncIO-specific benchmarks

{{ _common_data = globals().get("datal") }}
Run at: {{ =datetime.datetime.fromtimestamp(_common_data.run_at).strftime('%a %d %b %Y, %H:%M') }}    
Environment: {{ =benv }} (CPUs: {{ =_common_data.cpu }})    
Granian version: {{ =_common_data.granian }}

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

### Event loops

Comparison between different AsyncIO event loops on async protocols.

| Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- |
{{ _data = globals().get("datal", {}) }}
{{ for proto, pdata in _data.results["loops"].items(): }}
{{ for key, runs in pdata.items(): }}
{{ max_c, run = get_max_concurrency_run(runs) }}
| {{ =proto.upper() }} {{ =key }} (c{{ =max_c }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =fmt_ms(run["latency"]["avg"]) }} | {{ =fmt_ms(run["latency"]["max"]) }} |
{{ pass }}
{{ pass }}
{{ pass }}

### Task implementation

Comparison between Granian Rust AsyncIO task implementation and stdlib one on ASGI protocol.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
{{ for pykey in ["309", "310", "311"]: }}
{{ _data = globals().get(f"datat{pykey}") }}
{{ if not _data: }}
{{ continue }}
{{ for key, runs in _data.results["task_impl"].items(): }}
{{ max_c, run = get_max_concurrency_run(runs) }}
| {{ =_data.pyver }} | {{ =key }} (c{{ =max_c }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =fmt_ms(run["latency"]["avg"]) }} | {{ =fmt_ms(run["latency"]["max"]) }} |
{{ pass }}
{{ pass }}
