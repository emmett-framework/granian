# Granian benchmarks

{{ include './_helpers.tpl' }}

## VS 3rd party comparison

Run at: {{ =datetime.datetime.fromtimestamp(data.run_at).strftime('%a %d %b %Y, %H:%M') }}    
Environment: {{ =benv }} (CPUs: {{ =data.cpu }})    
Python version: {{ =data.pyver }}    
Granian version: {{ =data.granian }}

*Note: unless otherwise specified, all benchmarks are run with 1 server worker and 1 thread.*

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

{{ if wsdata := globals().get("wsdata"): }}
### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

{{ _data = wsdata.results["vs_ws"] }}
{{ include './_vs_ws_table.tpl' }}
{{ pass }}
