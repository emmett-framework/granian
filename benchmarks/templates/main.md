# Granian benchmarks

{{ include './_helpers.tpl' }}

Run at: {{ =datetime.datetime.fromtimestamp(data.run_at).strftime('%a %d %b %Y, %H:%M') }}    
Environment: {{ =benv }} (CPUs: {{ =data.cpu }})    
Python version: {{ =data.pyver }}    
Granian version: {{ =data.granian }}

### Methodology

Unless otherwise specified in the specific benchmark section, Granian is run:

- Using default configuration, thus:
  - 1 worker
  - 1 runtime thread
- With `--runtime-mode` set to `st` on ASGI and `mt` otherwise
- With `--http 1` flag
- With `--no-ws` flag
- With `uvloop` event-loop on async protocols

Tests are peformed using `oha` utility, with the concurrency specified in the specific test. The test run for 10 seconds, preceeded by a *primer* run at concurrency 8 for 4 seconds, and a *warmup* run at the maximum configured concurrency for the test for 3 seconds.

{{ _data = data.results["rsgi_body"] }}
{{ include './_rsgi.md' }}

{{ _data = data.results["interfaces"] }}
{{ include './_ifaces.md' }}

{{ _data = data.results["http2"] }}
{{ include './_http2.md' }}

{{ _data = data.results["files"] }}
{{ include './_files.md' }}

### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [AsyncIO-specific benchmarks](./asyncio.md)
- [Python versions](./pyver.md)

### 3rd party benchmarks

- [TFB](./external/tfb.md)
