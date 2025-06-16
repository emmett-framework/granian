{{ _target = {"async": _data.get('granian-rsgi', {}), "sync": _data.get('granian-wsgi', {})} }}
{{ groups = {"async": ["granian", "granian-rsgi", "fastwsgi-asgi", "robyn", "socketify.py-asgi-python3", "uvicorn"], "sync": ["granian-wsgi", "fastwsgi", "socketify.py-wsgi-python3", "uwsgi", "uwsgi-nginx-uwsgi"]} }}

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
