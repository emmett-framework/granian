# Granian benchmarks

{{ include './_helpers.tpl' }}

## Concurrency

Run at: {{ =datetime.datetime.fromtimestamp(data.run_at).strftime('%a %d %b %Y, %H:%M') }}    
Environment: {{ =benv }} (CPUs: {{ =data.cpu }})    
Python version: {{ =data.pyver }}    
Granian version: {{ =data.granian }}    

Same methodology of the main benchmarks applies.

The benchmark consists of an HTTP GET request returning a 1KB plain-text response (the response is a single static byte string).

### Workers

| Interface | Workers | Concurrency | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- |
{{ for interface in ["asgi", "rsgi", "wsgi"]: }}
{{ for run in data.results["concurrency_p"][interface].values(): }}
{{ res = run["res"][str(run["c"])] }}
| {{ =interface.upper() }} | {{ =run["p"] }} | {{ =run["c"] }} | {{ =res["requests"]["total"] }} | {{ =res["requests"]["rps"] }} | {{ =fmt_ms(res["latency"]["avg"]) }} | {{ =fmt_ms(res["latency"]["max"]) }} |
{{ pass }}
{{ pass }}

### Runtime threads

| Interface | Mode | Threads | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- | --- |
{{ for interface in ["asgi", "rsgi", "wsgi"]: }}
{{ for run in data.results["concurrency_t"][interface].values(): }}
{{ ckey = list(run["res"].keys())[0] }}
{{ res = run["res"][ckey] }}
| {{ =interface.upper() }} | {{ =run["m"].upper() }} | {{ =run["t"] }} | {{ =res["requests"]["total"] }} | {{ =res["requests"]["rps"] }} | {{ =fmt_ms(res["latency"]["avg"]) }} | {{ =fmt_ms(res["latency"]["max"]) }} |
{{ pass }}
{{ pass }}
