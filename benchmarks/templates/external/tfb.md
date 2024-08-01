# Granian 3rd party benchmarks

## TechEmpower frameworks benchmarks

[Repository](https://github.com/TechEmpower/FrameworkBenchmarks)    
[Website](http://www.techempower.com/benchmarks/)

Run at: {{ =datetime.datetime.fromtimestamp(data.run_at).strftime('%a %d %b %Y, %H:%M') }}    
Run ID: {{ =data.run }} ([visualize]({{ =data.url }}))

{{ _data, _labels = data.results, data.labels }}

### Plain text

{{ _test = "plaintext" }}
{{ include './_tfb_table.tpl' }}

### JSON

{{ _test = "json" }}
{{ include './_tfb_table.tpl' }}
