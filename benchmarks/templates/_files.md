## File responses

Comparison between Granian application protocols using ~50KB JPEG image.    
WSGI is not part of the benchmark since the protocol doesn't implement anything different from returning the file's contents directly.

Tests are run with `--runtime-blocking-threads 1`.

{{ include './_table.tpl' }}
