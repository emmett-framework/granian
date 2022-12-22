import datetime
import json
import multiprocessing
import os
import signal
import subprocess
import time

from contextlib import contextmanager

CPU = multiprocessing.cpu_count()
WRK_CONCURRENCIES = [CPU * 2 ** i for i in range(3, 7)]


@contextmanager
def app(name, procs=None, threads=None, thmode=None):
    procs = procs or 1
    threads = threads or 1
    thmode = thmode or "workers"
    proc = {
        "asgi": (
            "granian --interface asgi --no-ws --log-level warning --backlog 2048 "
            f"--workers {procs} --threads {threads} --threading-mode {thmode} "
            "app.asgi:app"
        ),
        "rsgi": (
            "granian --interface rsgi --no-ws --log-level warning --backlog 2048 "
            f"--workers {procs} --threads {threads} --threading-mode {thmode} "
            "app.rsgi:app"
        ),
        "wsgi": (
            "granian --interface wsgi --no-ws --log-level warning --backlog 2048 "
            f"--workers {procs} --threads {threads} --threading-mode {thmode} "
            "app.wsgi:app"
        ),
        "uvicorn_h11": (
            "uvicorn --interface asgi3 "
            "--no-access-log --log-level warning "
            f"--http h11 --workers {procs} app.asgi:app"
        ),
        "uvicorn_httptools": (
            "uvicorn --interface asgi3 "
            "--no-access-log --log-level warning "
            f"--http httptools --workers {procs} app.asgi:app"
        ),
        "hypercorn": (
            "hypercorn -b localhost:8000 -k uvloop --backlog 2048 "
            f"--workers {procs} app.asgi:app"
        ),
        "gunicorn": (
            "gunicorn -k meinheld.gmeinheld.MeinheldWorker "
            f"--workers {procs} app.wsgi:app"
        )
    }
    proc = subprocess.Popen(proc[name], shell=True, preexec_fn=os.setsid)
    time.sleep(2)
    yield proc
    os.killpg(os.getpgid(proc.pid), signal.SIGKILL)


def wrk(duration, concurrency, endpoint):
    threads = max(2, CPU // 2)
    proc = subprocess.run(
        f"wrk -d{duration}s -H \"Connection: keep-alive\" -t{threads} -c{concurrency} "
        f"-s wrk.lua http://localhost:8000/{endpoint}",
        shell=True,
        check=True,
        capture_output=True
    )
    data = proc.stderr.decode("utf8").split(",")
    return {
        "requests": {"total": data[1], "rps": data[2]},
        "latency": {"avg": data[11], "max": data[10], "stdev": data[12]}
    }


def benchmark(endpoint):
    results = {}
    # primer
    wrk(5, 8, endpoint)
    time.sleep(2)
    # warm up
    wrk(5, max(WRK_CONCURRENCIES), endpoint)
    time.sleep(3)
    # bench
    for concurrency in WRK_CONCURRENCIES:
        res = wrk(15, concurrency, endpoint)
        results[concurrency] = res
        time.sleep(3)
    time.sleep(2)
    return results


def concurrencies():
    nperm = sorted(set([1, 2, round(CPU / 2), CPU]))
    results = {}
    for interface in ["asgi", "rsgi", "wsgi"]:
        results[interface] = {}
        for idx, np in enumerate(nperm):
            tlimit = -idx if idx else None
            for nt in nperm[:tlimit]:
                key = f"P{np} T{nt}"
                with app(interface, np, nt):
                    results[interface][key] = benchmark("b")
    return results


def rsgi_body_type():
    results = {}
    with app("rsgi"):
        results["bytes small"] = benchmark("b")
        results["str small"] = benchmark("s")
        results["bytes big"] = benchmark("bb")
        results["str big"] = benchmark("ss")
    return results


def interfaces():
    results = {}
    with app("rsgi"):
        results["RSGI bytes"] = benchmark("b")
        results["RSGI str"] = benchmark("s")
    with app("asgi"):
        results["ASGI bytes"] = benchmark("b")
        results["ASGI str"] = benchmark("s")
    with app("wsgi"):
        results["WSGI bytes"] = benchmark("b")
        results["WSGI str"] = benchmark("s")
    return results


def vs_3rd_async():
    results = {}
    with app("asgi"):
        results["Granian ASGI"] = benchmark("b")
    with app("rsgi"):
        results["Granian RSGI"] = benchmark("b")
    with app("uvicorn_h11"):
        results["Uvicorn H11"] = benchmark("b")
    with app("uvicorn_httptools"):
        results["Uvicorn http-tools"] = benchmark("b")
    with app("hypercorn"):
        results["Hypercorn"] = benchmark("b")
    return results


def vs_3rd_sync():
    results = {}
    with app("wsgi"):
        results["Granian WSGI"] = benchmark("b")
    with app("gunicorn"):
        results["Gunicorn meinheld"] = benchmark("b")
    return results


def vs_3rd_maxc():
    results = {}
    procs = {
        "asgi": (int(os.environ.get("P_ASGI", 1)), int(os.environ.get("T_ASGI", 1))),
        "rsgi": (int(os.environ.get("P_RSGI", 1)), int(os.environ.get("T_RSGI", 1))),
        "wsgi": (int(os.environ.get("P_WSGI", 1)), int(os.environ.get("T_WSGI", 1))),
        "other": int(os.environ.get("P_OTH", CPU)),
    }
    with app("asgi", procs["asgi"][0], procs["asgi"][1]):
        results["Granian ASGI"] = benchmark("b")
    with app("rsgi", procs["rsgi"][0], procs["rsgi"][1]):
        results["Granian RSGI"] = benchmark("b")
    with app("wsgi", procs["wsgi"][0], procs["wsgi"][1]):
        results["Granian WSGI"] = benchmark("b")
    with app("uvicorn_httptools", procs["other"]):
        results["Uvicorn http-tools"] = benchmark("b")
    with app("hypercorn", procs["other"]):
        results["Hypercorn"] = benchmark("b")
    with app("gunicorn", procs["other"]):
        results["Gunicorn meinheld"] = benchmark("b")
    return results


def run():
    now = datetime.datetime.utcnow()
    results = {}
    if os.environ.get("BENCHMARK_BASE", "true") == "true":
        results["rsgi_body"] = rsgi_body_type()
        results["interfaces"] = interfaces()
    if os.environ.get("BENCHMARK_CONCURRENCIES") == "true":
        results["concurrencies"] = concurrencies()
    if os.environ.get("BENCHMARK_VSA") == "true":
        results["vs_async"] = vs_3rd_async()
    if os.environ.get("BENCHMARK_VSS") == "true":
        results["vs_sync"] = vs_3rd_sync()
    if os.environ.get("BENCHMARK_VSC") == "true":
        results["vs_maxc"] = vs_3rd_maxc()
    with open(f"results/data.json", "w") as f:
        f.write(json.dumps({
            "cpu": CPU,
            "run_at": now.isoformat(),
            "results": results
        }))


if __name__ == "__main__":
    run()
