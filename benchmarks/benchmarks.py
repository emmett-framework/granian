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

APPS = {
    "asgi": (
        "granian --interface asgi --log-level warning --backlog 2048 "
        "--no-ws --http {http} "
        "--workers {procs} --threads {threads}{bthreads} "
        "--threading-mode {thmode} app.asgi:app"
    ),
    "rsgi": (
        "granian --interface rsgi --log-level warning --backlog 2048 "
        "--no-ws --http {http} "
        "--workers {procs} --threads {threads}{bthreads} "
        "--threading-mode {thmode} app.rsgi:app"
    ),
    "wsgi": (
        "granian --interface wsgi --log-level warning --backlog 2048 "
        "--no-ws --http {http} "
        "--workers {procs} --threads {threads}{bthreads} "
        "--threading-mode {thmode} app.wsgi:app"
    ),
    "uvicorn_h11": (
        "uvicorn --interface asgi3 "
        "--no-access-log --log-level warning "
        "--http h11 --workers {procs} app.asgi:app"
    ),
    "uvicorn_httptools": (
        "uvicorn --interface asgi3 "
        "--no-access-log --log-level warning "
        "--http httptools --workers {procs} app.asgi:app"
    ),
    "hypercorn": (
        "hypercorn -b localhost:8000 -k uvloop --log-level warning --backlog 2048 "
        "--workers {procs} asgi:app.asgi:async_app"
    ),
    "gunicorn_gthread": "gunicorn --workers {procs} -k gthread app.wsgi:app",
    "gunicorn_gevent": "gunicorn --workers {procs} -k gevent app.wsgi:app",
    "uwsgi": (
        "uwsgi --http :8000 --master --processes {procs} --enable-threads "
        "--disable-logging --die-on-term --single-interpreter --lazy-apps "
        "--wsgi-file app/wsgi.py --callable app"
    )
}


@contextmanager
def app(name, procs=None, threads=None, bthreads=None, thmode=None, http="1"):
    procs = procs or 1
    threads = threads or 1
    bthreads = f" --blocking-threads {bthreads}" if bthreads else ""
    thmode = thmode or "workers"
    proc_cmd = APPS[name].format(
        procs=procs,
        threads=threads,
        bthreads=bthreads,
        thmode=thmode,
        http=http,
    )
    proc = subprocess.Popen(proc_cmd, shell=True, preexec_fn=os.setsid)
    time.sleep(2)
    yield proc
    os.killpg(os.getpgid(proc.pid), signal.SIGKILL)


def wrk(duration, concurrency, endpoint, post=False, h2=False):
    cmd_parts = [
        "rewrk",
        f"-c {concurrency}",
        f"-d {duration}s",
        "--json",
    ]
    if h2:
        cmd_parts.append("--http2")
    else:
        cmd_parts.append("-H \"Connection: Keep-Alive\"")
        cmd_parts.append("-H \"Keep-Alive: timeout=60'\"")
    if post:
        cmd_parts.append("-m post")
        cmd_parts.append("-H \"Content-Type: text/plain; charset=utf-8\"")
        cmd_parts.append("-H \"Content-Length: 4\"")
        cmd_parts.append("-b \"test\"")
    cmd_parts.append(f"-h http://127.0.0.1:8000/{endpoint}")
    try:
        proc = subprocess.run(
            " ".join(cmd_parts),
            shell=True,
            check=True,
            capture_output=True,
        )
        data = json.loads(proc.stdout.decode("utf8"))
        return {
            "requests": {
                "total": data["requests_total"],
                "rps": round(data["requests_avg"] or 0)
            },
            "latency": {
                "avg": data["latency_avg"],
                "max": data["latency_max"],
                "stdev": data["latency_std_deviation"]
            },
        }
    except Exception as e:
        print(f"WARN: got exception {e} while loading rewrk data")
        return {
            "requests": {"total": 0, "rps": 0},
            "latency": {"avg": None, "max": None, "stdev": None},
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


def concurrencies():
    nperm = sorted(set([1, 2, 4, round(CPU / 2), CPU]))
    results = {"wsgi": {}}
    for interface in ["asgi", "rsgi", "wsgi"]:
        results[interface] = {}
        for np in nperm:
            for nt in [1, 2, 4]:
                for threading_mode in ["workers", "runtime"]:
                    key = f"P{np} T{nt} {threading_mode[0]}th"
                    with app(interface, np, nt, bthreads=1, thmode=threading_mode):
                        print(f"Bench concurrencies - [{interface}] {threading_mode} {np}:{nt}")
                        results[interface][key] = {
                            "m": threading_mode,
                            "p": np,
                            "t": nt,
                            "res": benchmark("b", concurrencies=[128, 512, 1024, 2048])
                        }
    return results


def rsgi_body_type():
    results = {}
    benches = {"bytes small": "b", "str small": "s", "bytes big": "bb", "str big": "ss"}
    for title, route in benches.items():
        with app("rsgi"):
            results[title] = benchmark(route)
    return results


def interfaces():
    results = {}
    benches = {"bytes": ("b", {}), "str": ("s", {}), "echo": ("echo", {"post": True})}
    for interface in ["rsgi", "asgi", "wsgi"]:
        for key, bench_data in benches.items():
            route, opts = bench_data
            with app(interface, bthreads=1):
                results[f"{interface.upper()} {key}"] = benchmark(route, **opts)
    return results


def http2():
    results = {}
    benches = {"[GET]": ("b", {}), "[POST]": ("echo", {"post": True})}
    for http2 in [False, True]:
        for key, bench_data in benches.items():
            route, opts = bench_data
            h = "2" if http2 else "1"
            with app("rsgi", http=h):
                results[f"HTTP/{h} {key}"] = benchmark(route, h2=http2, **opts)
    return results


def files():
    results = {}
    with app("rsgi", bthreads=1):
        results["RSGI"] = benchmark("fp")
    with app("asgi", bthreads=1):
        results["ASGI"] = benchmark("fb")
        results["ASGI pathsend"] = benchmark("fp")
    return results


def vs_asgi():
    results = {}
    benches = {"[GET]": ("b", {}), "[POST]": ("echo", {"post": True})}
    for fw in ["granian_asgi", "uvicorn_h11", "uvicorn_httptools", "hypercorn"]:
        for key, bench_data in benches.items():
            route, opts = bench_data
            fw_app = fw.split("_")[1] if fw.startswith("granian") else fw
            title = " ".join(item.title() for item in fw.split("_"))
            with app(fw_app):
                results[f"{title} {key}"] = benchmark(route, **opts)
    return results


def vs_wsgi():
    results = {}
    benches = {"[GET]": ("b", {}), "[POST]": ("echo", {"post": True})}
    for fw in ["granian_wsgi", "gunicorn_gthread", "gunicorn_gevent", "uwsgi"]:
        for key, bench_data in benches.items():
            route, opts = bench_data
            fw_app = fw.split("_")[1] if fw.startswith("granian") else fw
            title = " ".join(item.title() for item in fw.split("_"))
            with app(fw_app, bthreads=1):
                results[f"{title} {key}"] = benchmark(route, **opts)
    return results


def vs_http2():
    results = {}
    benches = {"[GET]": ("b", {}), "[POST]": ("echo", {"post": True})}
    for fw in ["granian_asgi", "hypercorn"]:
        for key, bench_data in benches.items():
            route, opts = bench_data
            fw_app = fw.split("_")[1] if fw.startswith("granian") else fw
            title = " ".join(item.title() for item in fw.split("_"))
            with app(fw_app, http="2"):
                results[f"{title} {key}"] = benchmark(route, h2=True, **opts)
    return results


def vs_files():
    results = {}
    with app("asgi", bthreads=1):
        results["Granian (pathsend)"] = benchmark("fp")
    for fw in ["uvicorn_h11", "uvicorn_httptools", "hypercorn"]:
        title = " ".join(item.title() for item in fw.split("_"))
        with app(fw):
            results[title] = benchmark("fb")
    return results


def vs_io():
    results = {}
    benches = {"10ms": ("io10", {}), "100ms": ("io100", {})}
    for fw in [
        "granian_rsgi",
        "granian_asgi",
        "granian_wsgi",
        "uvicorn_httptools",
        "hypercorn",
        "gunicorn_gevent",
        "uwsgi",
    ]:
        for key, bench_data in benches.items():
            route, opts = bench_data
            fw_app = fw.split("_")[1] if fw.startswith("granian") else fw
            title = " ".join(item.title() for item in fw.split("_"))
            with app(fw_app):
                results[f"{title} {key}"] = benchmark(route, **opts)
    return results


def _granian_version():
    import granian
    return granian.__version__


def run():
    now = datetime.datetime.utcnow()
    results = {}
    if os.environ.get("BENCHMARK_BASE", "true") == "true":
        results["rsgi_body"] = rsgi_body_type()
        results["interfaces"] = interfaces()
        results["http2"] = http2()
        results["files"] = files()
    if os.environ.get("BENCHMARK_CONCURRENCIES") == "true":
        results["concurrencies"] = concurrencies()
    if os.environ.get("BENCHMARK_VS") == "true":
        results["vs_asgi"] = vs_asgi()
        results["vs_wsgi"] = vs_wsgi()
        results["vs_http2"] = vs_http2()
        results["vs_files"] = vs_files()
        results["vs_io"] = vs_io()
    with open("results/data.json", "w") as f:
        pyver = sys.version_info
        f.write(json.dumps({
            "cpu": CPU,
            "run_at": int(now.timestamp()),
            "pyver": f"{pyver.major}.{pyver.minor}",
            "results": results,
            "granian": _granian_version()
        }))


if __name__ == "__main__":
    run()
