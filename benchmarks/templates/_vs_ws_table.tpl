| Clients | Server | Send throughput | Receive throughput | Combined throughput |
| --- | --- | --- | --- | --- |
{{ _skeys = list(_data.keys()) }}
{{ for concur in [8, 16, 32]: }}
{{ for key in _skeys: }}
{{ run = _data[key][str(concur)] }}
{{ if run["throughput"]["sum"]: }}
| {{ =concur }} | {{ =key }} | {{ =round(run["throughput"]["send"]) }} | {{ =round(run["throughput"]["recv"]) }} | {{ =round(run["throughput"]["sum"]) }} |
{{ else: }}
| {{ =concur }} | {{ =key }} | N/A | N/A | N/A |
{{ pass }}
{{ pass }}
{{ pass }}
