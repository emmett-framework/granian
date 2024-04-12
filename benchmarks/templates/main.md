# Granian benchmarks

{{ include './_helpers.tpl' }}

Run at: {{ =datetime.datetime.fromtimestamp(data.run_at).strftime('%a %d %b %Y, %H:%M') }}    
Environment: {{ =benv }} (CPUs: {{ =data.cpu }})    
Python version: {{ =data.pyver }}    
Granian version: {{ =data.granian }}    

{{ _data = data.results["rsgi_body"] }}
{{ include './_rsgi.tpl' }}

{{ _data = data.results["interfaces"] }}
{{ include './_ifaces.tpl' }}

{{ _data = data.results["http2"] }}
{{ include './_http2.tpl' }}

{{ _data = data.results["files"] }}
{{ include './_files.tpl' }}

### Other benchmarks

- [Versus 3rd party servers](./vs.md)
- [Concurrency](./concurrency.md)
{{ if False: }}
- [Python versions](./pyver.md)
{{ pass }}
