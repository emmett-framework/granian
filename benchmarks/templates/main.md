# Granian benchmarks

{{ include './_helpers.tpl' }}

Run at: {{ =datetime.datetime.fromtimestamp(data.run_at).strftime('%a %d %b %Y, %H:%M') }}    
Environment: {{ =benv }} (CPUs: {{ =data.cpu }})    
Python version: {{ =data.pyver }}    
Granian version: {{ =data.granian }}

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

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
