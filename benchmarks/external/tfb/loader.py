import datetime
import json
import urllib.request

from selectolax.parser import HTMLParser


TFB_URL = 'https://tfb-status.techempower.com'
FW_MAP = {
    'fastwsgi-asgi': 'FastWSGI (ASGI)',
    'granian': 'Granian (ASGI)',
    'granian-rsgi': 'Granian (RSGI)',
    'granian-wsgi': 'Granian (WSGI)',
    'socketify.py-asgi-python3': 'Socketify (ASGI)',
    'socketify.py-wsgi-python3': 'Socketify (WSGI)',
    'uvicorn': 'Uvicorn (httptools)',
    'uwsgi': 'uWSGI',
    'uwsgi-nginx-uwsgi': 'uWSGI + Nginx',
}


def get_runs():
    with urllib.request.urlopen(TFB_URL) as res:  # noqa: S310
        data = res.read()
    return data


def find_last_run(data):
    html = HTMLParser(data)
    for node in html.css('tr'):
        if 'estimated' not in node.text():
            return node.attributes['data-uuid']


def get_run_meta(runid):
    with urllib.request.urlopen(TFB_URL + f'/results/{runid}') as res:  # noqa: S310
        data = res.read()

    html = HTMLParser(data)
    target = None
    visualize_url = html.css_first('a').attributes['href']

    for node in html.css('a'):
        if 'unzip' in node.attributes['href']:
            target = node.attributes['href']
            break

    assert target is not None
    with urllib.request.urlopen(TFB_URL + f'{target}/results') as res:  # noqa: S310
        data = res.read()
    node = HTMLParser(data).css_first('.fileName')
    return {
        'date': datetime.datetime.strptime(node.text()[:-1], '%Y%m%d%H%M%S'),
        'target': node.attributes['href'],
        'visualize': visualize_url,
    }


def extract_run_data(data: str):
    rps = []
    for line in filter(lambda s: 'Requests/sec:' in s, data.splitlines()):
        rps.append(float(line.split('Requests/sec:')[-1].strip()))
    return round(max(rps))


def get_run_results(path, fw):
    rv = {}
    for bench in ['json', 'plaintext']:
        try:
            with urllib.request.urlopen(TFB_URL + f'{path}/{fw}/{bench}/raw.txt') as res:  # noqa: S310
                rv[bench] = extract_run_data(res.read().decode('utf-8'))
        except Exception:
            pass
    return rv


def run():
    runs = get_runs()
    run_uuid = find_last_run(runs)
    run_meta = get_run_meta(run_uuid)
    res = {
        'run': run_uuid,
        'results': {},
        'labels': {},
        'run_at': int(run_meta['date'].timestamp()),
        'url': run_meta['visualize'],
    }
    # NOTE: would be nice to have updated gunicorn, hypercorn
    for fw in [
        'fastwsgi',
        'fastwsgi-asgi',
        'granian',
        'granian-rsgi',
        'granian-wsgi',
        'robyn',
        'socketify.py-asgi-python3',
        'socketify.py-wsgi-python3',
        'uvicorn',
        'uwsgi',
        # NOTE: wsgi is gunicorn, but is Python 3.6 based with meinheld, no more avail
        # "wsgi",
        'uwsgi-nginx-uwsgi',
    ]:
        fw_label = FW_MAP.get(fw) or fw.title()
        res['results'][fw] = get_run_results(run_meta['target'], fw)
        res['labels'][fw] = fw_label
    print(json.dumps(res))


if __name__ == '__main__':
    run()
