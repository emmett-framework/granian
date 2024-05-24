# Granian benchmarks

{{ include './_helpers.tpl' }}

## VS 3rd party comparison

Run at: {{ =datetime.datetime.fromtimestamp(data.run_at).strftime('%a %d %b %Y, %H:%M') }}    
Environment: {{ =benv }} (CPUs: {{ =data.cpu }})    
Python version: {{ =data.pyver }}    
Granian version: {{ =data.granian }}    

### ASGI

{{ _data = data.results["vs_asgi"] }}
{{ include './_vs_table.tpl' }}

### WSGI

{{ _data = data.results["vs_wsgi"] }}
{{ include './_vs_table.tpl' }}

### HTTP/2

{{ _data = data.results["vs_http2"] }}
{{ include './_vs_table.tpl' }}

### ASGI file responses

{{ _data = data.results["vs_files"] }}
{{ include './_vs_table.tpl' }}

### Long I/O

Plain text 4 bytes response comparison simulating *long* I/O waits (10ms and 100ms).

{{ _data = data.results["vs_io"] }}
{{ include './_vs_table.tpl' }}
