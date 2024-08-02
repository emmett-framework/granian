{{ _target = {"async": _data.get('granian-rsgi', {}), "sync": _data.get('granian-wsgi', {})} }}
{{ groups = {"async": ["fastwsgi-asgi", "granian", "granian-rsgi", "robyn", "uvicorn"], "sync": ["fastwsgi", "granian-wsgi", "uwsgi", "uwsgi-nginx-uwsgi"]} }}

{{ for group in ["async", "sync"]: }}
#### {{ =group.title() }}

| Server | RPS | Change (rate) |
| --- | --- | --- |
{{ for key in sorted(set(groups[group]) & set(_data.keys())): }}
{{ vals = _data[key] }}
{{ if _test not in vals: }}
{{ continue }}
| {{ =_labels[key] }} | {{ =vals[_test] }} | {{ =round(vals[_test] / _target[group].get(_test, 1), 2) }} |
{{ pass }}

{{ pass }}
