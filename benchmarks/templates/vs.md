# Granian benchmarks

{{ include './_helpers.tpl' }}

## VS 3rd party comparison

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

Tests are peformed using `oha` utility, with the concurrency specified in the specific test. The test run for 10 seconds, preceeded by a *primer* run at concurrency 8 for 4 seconds, and a *warmup* run at the maximum configured concurrency for the test for 3 seconds.

All the async servers – including Granian – are using `uvloop` for the asyncio event-loop implementation.

All the reported 3rd party servers were installed using the latest available version at the time of the run.

The *get* benchmark consists of an HTTP GET request returning a 10KB plain-text response (the response is a single static byte string).

The *echo* benchmark consists of an HTTP POST request with a 10KB plain-text body, which will be *streamed* back (the iteration happens in chunks with a dimension depending on the underlying protocol).

### ASGI

{{ _data = data.results["vs_asgi"] }}
{{ include './_vs_table.tpl' }}

### WSGI

Granian is run with `--blocking-threads 1`.

{{ _data = data.results["vs_wsgi"] }}
{{ include './_vs_table.tpl' }}

### HTTP/2

Granian is run with `--http 2` and `--runtime-threads 2`.

{{ _data = data.results["vs_http2"] }}
{{ include './_vs_table.tpl' }}

### ASGI file responses

The benchmark performs an HTTP GET request returning a ~50KB JPEG image. While on *pathsend* the implementation is entirely provided by the underlying protocol, in all the other cases the entirety of the file is read and collected in memory and thus returned as a single byte string.

Granian is run with `--runtime-blocking-threads 1`.

{{ _data = data.results["vs_files"] }}
{{ include './_vs_table.tpl' }}

### Long I/O

Plain text response comparison simulating *long* I/O waits (10ms and 100ms).

{{ _data = data.results["vs_io"] }}
{{ include './_vs_table.tpl' }}

{{ if wsdata := globals().get("wsdata"): }}
### Websockets

Websocket broadcasting comparison with concurrent clients sending a predefined amount of messages and receiving those messages from all the connected clients. The benchmark takes the time required for the test to run and compute the relevant throughput (in messages per second).

Granian is run with `--ws`.

{{ _data = wsdata.results["vs_ws"] }}
{{ include './_vs_ws_table.tpl' }}
{{ pass }}
