import datetime
import json
import multiprocessing
import os
import signal
import subprocess
import sys
import time
from contextlib import contextmanager


CPU = multiprocessing.cpu_count()
WRK_CONCURRENCIES = [64, 128, 256, 512]
WS_CONCURRENCIES = [(8, 20_000), (16, 10_000), (32, 5000), (64, 2500)]

APPS = {
    'asgi': (
        'granian --interface asgi --log-level warning --backlog 2048 '
        '{wsmode}--http {http} --loop {loop} --task-impl {timpl} '
        '--workers {procs} --runtime-threads {threads}{bthreads} '
        '--runtime-mode {thmode} {app}.asgi:app'
    ),
    'rsgi': (
        'granian --interface rsgi --log-level warning --backlog 2048 '
        '{wsmode}--http {http} --loop {loop} --task-impl {timpl} '
        '--workers {procs} --runtime-threads {threads}{bthreads} '
        '--runtime-mode {thmode} {app}.rsgi:app'
    ),
    'wsgi': (
        'granian --interface wsgi --log-level warning --backlog 2048 '
        '{wsmode}--http {http} '
        '--workers {procs} --runtime-threads {threads}{bthreads} '
        '--runtime-mode {thmode} app.wsgi:app'
    ),
    'uvicorn_h11': (
        'uvicorn --interface asgi3 --no-access-log --log-level warning --http h11 --workers {procs} {app}.asgi:app'
    ),
    'uvicorn_httptools': (
        'uvicorn --interface asgi3 '
        '--no-access-log --log-level warning '
        '--http httptools --workers {procs} {app}.asgi:app'
    ),
    'hypercorn': (
        'hypercorn -b localhost:8000 -k uvloop --log-level warning --backlog 2048 '
        '--workers {procs} asgi:{app}.asgi:async_app'
    ),
    'gunicorn_gthread': 'gunicorn --workers {procs} -k gthread app.wsgi:app',
    'gunicorn_gevent': 'gunicorn --workers {procs} -k gevent app.wsgi:app',
    'uwsgi': (
        'uwsgi --http :8000 --master --processes {procs} --enable-threads '
        '--disable-logging --die-on-term --single-interpreter --lazy-apps '
        '--wsgi-file app/wsgi.py --callable app'
    ),
}


@contextmanager
def app(
    name,
    procs=None,
    threads=None,
    bthreads=None,
    thmode=None,
    loop='uvloop',
    timpl='asyncio',
    http='1',
    ws=False,
    app_path='app',
):
    procs = procs or 1
    threads = threads or 1
    bthreads_flag = 'blocking-threads' if name == 'wsgi' else 'runtime-blocking-threads'
    bthreads = f' --{bthreads_flag} {bthreads}' if bthreads else ''
    thmode = thmode or 'st'
    wsmode = '--no-ws ' if not ws else ''
    exc_prefix = os.environ.get('BENCHMARK_EXC_PREFIX')
    proc_cmd = APPS[name].format(
        app=app_path,
        procs=procs,
        threads=threads,
        bthreads=bthreads,
        thmode=thmode,
        loop=loop,
        timpl=timpl,
        http=http,
        wsmode=wsmode,
    )
    if exc_prefix:
        proc_cmd = f'{exc_prefix}/{proc_cmd}'
    proc = subprocess.Popen(proc_cmd, shell=True, preexec_fn=os.setsid)  # noqa: S602
    time.sleep(2)
    yield proc
    os.killpg(os.getpgid(proc.pid), signal.SIGKILL)


def wrk(duration, concurrency, endpoint, post=None, h2=False):
    cmd_parts = [
        'rewrk',
        f'-c {concurrency}',
        f'-d {duration}s',
        '--json',
    ]
    if h2:
        cmd_parts.append('--http2')
    else:
        cmd_parts.append('-H "Connection: Keep-Alive"')
        cmd_parts.append('-H "Keep-Alive: timeout=60\'"')
    post_body = ''
    if post:
        post_body = 'x' * post
        cmd_parts.append('-m post')
        cmd_parts.append('-H "Content-Type: text/plain; charset=utf-8"')
        cmd_parts.append(f'-H "Content-Length: {post}"')
        cmd_parts.append(f'-b "{post_body}"')
    cmd_parts.append(f'-h http://127.0.0.1:8000/{endpoint}')
    try:
        proc = subprocess.run(  # noqa: S602
            ' '.join(cmd_parts),
            shell=True,
            check=True,
            capture_output=True,
        )
        data = json.loads(proc.stdout.decode('utf8'))
        return {
            'requests': {'total': data['requests_total'], 'rps': round(data['requests_avg'] or 0)},
            'latency': {'avg': data['latency_avg'], 'max': data['latency_max'], 'stdev': data['latency_std_deviation']},
        }
    except Exception as e:
        print(f'WARN: got exception {e} while loading rewrk data')
        return {
            'requests': {'total': 0, 'rps': 0},
            'latency': {'avg': None, 'max': None, 'stdev': None},
        }


def wsb(concurrency, msgs):
    exc_prefix = os.environ.get('BENCHMARK_EXC_PREFIX')
    cmd_parts = [
        f'{exc_prefix}/python' if exc_prefix else 'python',
        os.path.join(os.path.dirname(__file__), 'ws/benchmark.py'),
    ]
    env = dict(os.environ)
    try:
        proc = subprocess.run(  # noqa: S602
            ' '.join(cmd_parts),
            shell=True,
            check=True,
            capture_output=True,
            env={'BENCHMARK_CONCURRENCY': str(concurrency), 'BENCHMARK_MSGNO': str(msgs), **env},
        )
        return json.loads(proc.stdout.decode('utf8'))
    except Exception as e:
        print(f'WARN: got exception {e} while loading wsbench data')
        return {
            'timings': {
                'recv': {'avg': 0, 'max': 0, 'min': 0},
                'send': {'avg': 0, 'max': 0, 'min': 0},
                'sum': {'avg': 0, 'max': 0, 'min': 0},
                'all': {'avg': 0, 'max': 0, 'min': 0},
            },
            'throughput': {
                'recv': 0,
                'send': 0,
                'all': 0,
                'sum': 0,
            },
        }


def benchmark(endpoint, post=False, h2=False, concurrencies=None):
    concurrencies = concurrencies or WRK_CONCURRENCIES
    results = {}
    # primer
    wrk(4, 8, endpoint, post=post, h2=h2)
    time.sleep(1)
    # warm up
    wrk(3, max(concurrencies), endpoint, post=post, h2=h2)
    time.sleep(2)
    # bench
    for concurrency in concurrencies:
        res = wrk(10, concurrency, endpoint, post=post, h2=h2)
        results[concurrency] = res
        time.sleep(3)
    time.sleep(1)
    return results


def benchmark_ws(concurrencies=None):
    concurrencies = concurrencies or WS_CONCURRENCIES
    results = {}
    # bench
    for concurrency, msgs in concurrencies:
        res = wsb(concurrency, msgs)
        results[concurrency] = res
        time.sleep(2)
    return results


def concurrencies():
    nperm = sorted({1, 2, 4, round(CPU / 2), CPU})
    results = {'wsgi': {}}
    for interface in ['asgi', 'rsgi', 'wsgi']:
        results[interface] = {}
        for np in nperm:
            for nt in [1, 2, 4]:
                for threading_mode in ['st', 'mt']:
                    key = f'P{np} T{nt} {threading_mode.upper()}'
                    with app(interface, np, nt, bthreads=1, thmode=threading_mode):
                        print(f'Bench concurrencies - [{interface}] {threading_mode} {np}:{nt}')
                        results[interface][key] = {
                            'm': threading_mode,
                            'p': np,
                            't': nt,
                            'res': benchmark('b', concurrencies=[128, 512, 1024, 2048]),
                        }
    return results


def rsgi_body_type():
    results = {}
    benches = {'bytes 10B': 'b10', 'str 10B': 's10', 'bytes 100KB': 'b100k', 'str 100KB': 's100k'}
    for title, route in benches.items():
        with app('rsgi'):
            results[title] = benchmark(route)
    return results


def interfaces():
    results = {}
    benches = {
        'get 1KB': ('b1k', {}, {'bthreads': 1}),
        'echo 1KB': ('echo', {'post': 1024}, {'bthreads': 1}),
        'echo 100KB (iter)': ('echoi', {'post': 100 * 1024}, {}),
    }
    for interface in ['rsgi', 'asgi', 'wsgi']:
        for key, bench_data in benches.items():
            route, opts, run_opts = bench_data
            with app(interface, **run_opts):
                results[f'{interface.upper()} {key}'] = benchmark(route, **opts)
    return results


def http2():
    results = {}
    benches = {'get 1KB': ('b1k', {}), 'echo 1KB': ('echo', {'post': 1024})}
    for http2 in [False, True]:
        for key, bench_data in benches.items():
            route, opts = bench_data
            h = '2' if http2 else '1'
            with app('rsgi', http=h):
                results[f'HTTP/{h} {key}'] = benchmark(route, h2=http2, **opts)
    return results


def files():
    results = {}
    with app('rsgi'):
        results['RSGI'] = benchmark('fp')
    with app('asgi'):
        results['ASGI'] = benchmark('fb')
        results['ASGI pathsend'] = benchmark('fp')
    return results


def loops():
    results = {'asgi': {}, 'rsgi': {}}
    for interface in ['asgi', 'rsgi']:
        with app(interface, loop='asyncio'):
            results[interface]['asyncio get 10KB'] = benchmark('b10k')
            results[interface]['asyncio echo 10KB (iter)'] = benchmark('echoi', post=10 * 1024)
        with app(interface, loop='rloop'):
            results[interface]['rloop get 10KB'] = benchmark('b10k')
            results[interface]['rloop echo 10KB (iter)'] = benchmark('echoi', post=10 * 1024)
        with app(interface, loop='uvloop'):
            results[interface]['uvloop get 10KB'] = benchmark('b10k')
            results[interface]['uvloop echo 10KB (iter)'] = benchmark('echoi', post=10 * 1024)
    return results


def task_impl():
    results = {}
    with app('asgi', loop='asyncio', timpl='asyncio'):
        results['asyncio get 10KB'] = benchmark('b10k')
        results['asyncio echo 10KB (iter)'] = benchmark('echoi', post=10 * 1024)
    with app('asgi', loop='asyncio', timpl='rust'):
        results['rust get 10KB'] = benchmark('b10k')
        results['rust echo 10KB (iter)'] = benchmark('echoi', post=10 * 1024)
    return results


def vs_asgi():
    results = {}
    benches = {'get 10KB': ('b10k', {}), 'echo 10KB (iter)': ('echoi', {'post': 10 * 1024})}
    for fw in ['granian_asgi', 'uvicorn_h11', 'uvicorn_httptools', 'hypercorn']:
        for key, bench_data in benches.items():
            route, opts = bench_data
            fw_app = fw.split('_')[1] if fw.startswith('granian') else fw
            title = ' '.join(item.title() for item in fw.split('_'))
            with app(fw_app):
                results[f'{title} {key}'] = benchmark(route, **opts)
    return results


def vs_wsgi():
    results = {}
    benches = {'get 10KB': ('b10k', {}), 'echo 10KB (iter)': ('echoi', {'post': 10 * 1024})}
    for fw in ['granian_wsgi', 'gunicorn_gthread', 'gunicorn_gevent', 'uwsgi']:
        for key, bench_data in benches.items():
            route, opts = bench_data
            fw_app = fw.split('_')[1] if fw.startswith('granian') else fw
            title = ' '.join(item.title() for item in fw.split('_'))
            with app(fw_app, bthreads=1):
                results[f'{title} {key}'] = benchmark(route, **opts)
    return results


def vs_http2():
    results = {}
    benches = {'get 10KB': ('b10k', {}), 'echo 10KB (iter)': ('echoi', {'post': 10 * 1024})}
    for fw in ['granian_asgi', 'hypercorn']:
        for key, bench_data in benches.items():
            route, opts = bench_data
            fw_app = fw.split('_')[1] if fw.startswith('granian') else fw
            title = ' '.join(item.title() for item in fw.split('_'))
            with app(fw_app, http='2'):
                results[f'{title} {key}'] = benchmark(route, h2=True, **opts)
    return results


def vs_files():
    results = {}
    with app('asgi'):
        results['Granian (pathsend)'] = benchmark('fp')
    for fw in ['uvicorn_h11', 'uvicorn_httptools', 'hypercorn']:
        title = ' '.join(item.title() for item in fw.split('_'))
        with app(fw):
            results[title] = benchmark('fb')
    return results


def vs_io():
    results = {}
    benches = {'10ms': ('io10', {}), '100ms': ('io100', {})}
    for fw in [
        'granian_rsgi',
        'granian_asgi',
        'granian_wsgi',
        'uvicorn_httptools',
        'hypercorn',
        'gunicorn_gevent',
        'uwsgi',
    ]:
        for key, bench_data in benches.items():
            route, opts = bench_data
            fw_app = fw.split('_')[1] if fw.startswith('granian') else fw
            title = ' '.join(item.title() for item in fw.split('_'))
            with app(fw_app):
                results[f'{title} {key}'] = benchmark(route, **opts)
    return results


def vs_ws():
    results = {}
    for fw in [
        'granian_rsgi',
        'granian_asgi',
        'uvicorn_h11',
        'hypercorn',
    ]:
        fw_app = fw.split('_')[1] if fw.startswith('granian') else fw
        title = ' '.join(item.title() for item in fw.split('_'))
        with app(fw_app, ws=True, app_path='ws.app'):
            results[title] = benchmark_ws()
    return results


def _granian_version():
    import granian

    return granian.__version__


def run():
    all_benchmarks = {
        'rsgi_body': rsgi_body_type,
        'interfaces': interfaces,
        'http2': http2,
        'files': files,
        'loops': loops,
        'task_impl': task_impl,
        'concurrencies': concurrencies,
        'vs_asgi': vs_asgi,
        'vs_wsgi': vs_wsgi,
        'vs_http2': vs_http2,
        'vs_files': vs_files,
        'vs_io': vs_io,
        'vs_ws': vs_ws,
    }
    inp_benchmarks = sys.argv[1:] or ['base']
    if 'base' in inp_benchmarks:
        inp_benchmarks.remove('base')
        inp_benchmarks.extend(['rsgi_body', 'interfaces', 'http2', 'files'])
    if 'asyncio' in inp_benchmarks:
        inp_benchmarks.remove('asyncio')
        inp_benchmarks.extend(['loops', 'task_impl'])
    if 'vs' in inp_benchmarks:
        inp_benchmarks.remove('vs')
        inp_benchmarks.extend(['vs_asgi', 'vs_wsgi', 'vs_http2', 'vs_files', 'vs_io'])
    run_benchmarks = set(inp_benchmarks) & set(all_benchmarks.keys())

    now = datetime.datetime.utcnow()
    results = {}
    for benchmark_key in run_benchmarks:
        runner = all_benchmarks[benchmark_key]
        results[benchmark_key] = runner()

    with open('results/data.json', 'w') as f:
        pyver = sys.version_info
        f.write(
            json.dumps(
                {
                    'cpu': CPU,
                    'run_at': int(now.timestamp()),
                    'pyver': f'{pyver.major}.{pyver.minor}',
                    'results': results,
                    'granian': _granian_version(),
                }
            )
        )


if __name__ == '__main__':
    run()
