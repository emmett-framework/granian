# Granian benchmarks

{{ include './_helpers.tpl' }}

## Python versions

{{ _common_data = globals().get(f"data{pyvb}") }}
Run at: {{ =datetime.datetime.fromtimestamp(_common_data.run_at).strftime('%a %d %b %Y, %H:%M') }}    
Environment: {{ =benv }} (CPUs: {{ =_common_data.cpu }})    
Granian version: {{ =_common_data.granian }}    

Comparison between different Python versions of Granian application protocols using 4bytes plain text response.    
Bytes and string response are reported for every protocol just to report the difference with RSGI protocol.    
ASGI and WSGI responses are always returned as bytes by the application.    
The "echo" request is a 4bytes POST request responding with the same body.

| Python version | Request | Total requests | RPS | avg latency | max latency |
| --- | --- | --- | --- | --- | --- |
{{ for pykey in ["310", "311", "312", "313"]: }}
{{ _data = globals().get(f"data{pykey}") }}
{{ for key, runs in _data.results["interfaces"].items(): }}
{{ max_c, run = get_max_concurrency_run(runs) }}
| {{ =_data.pyver }} | {{ =key }} (c{{ =max_c }}) | {{ =run["requests"]["total"] }} | {{ =run["requests"]["rps"] }} | {{ =fmt_ms(run["latency"]["avg"]) }} | {{ =fmt_ms(run["latency"]["max"]) }} |
{{ pass }}
{{ pass }}
