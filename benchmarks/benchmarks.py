import datetime
import json
import math
import multiprocessing
import os
import signal
import subprocess
import time

from contextlib import contextmanager

CPU = multiprocessing.cpu_count()
CONCURRENCIES = [CPU * 2 ** i for i in range(3, 7)]


@contextmanager
def app(name, procs = None, threads = None):
    procs = procs or CPU
    # threads = threads or CPU // 2
    threads = threads or CPU * 2
    proc = {
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
        "asgi": f"python app/asgi.py {procs} {threads}",
        "rsgi": f"python app/rsgi.py {procs} {threads}"
    }
    proc = subprocess.Popen(proc[name], shell=True, preexec_fn=os.setsid)
    time.sleep(2)
    yield proc
    os.killpg(os.getpgid(proc.pid), signal.SIGKILL)


def wrk(duration, concurrency, endpoint):
    threads = max(4, CPU // 2)
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
    time.sleep(5)
    # warmup
    wrk(10, max(CONCURRENCIES), endpoint)
    time.sleep(5)
    # bench
    for concurrency in CONCURRENCIES:
        res = wrk(15, concurrency, endpoint)
        results[concurrency] = res
        time.sleep(3)
    time.sleep(5)
    return results


def procs_threads():
    results = {}
    for procs in [2 ** i for i in range(0, math.ceil(math.log2(CPU)) + 1)]:
        for threads in [2 ** i for i in range(0, math.ceil(math.log(CPU)) + 3)]:
            with app("rsgi", procs, threads):
                results[f"{procs} procs - {threads} threads"] = benchmark("b")
    return results


def rsgi_body_type():
    results = {}
    with app("rsgi"):
        results["bytes small"] = benchmark("b")
        results["str small"] = benchmark("s")
        results["bytes big"] = benchmark("bb")
        results["str big"] = benchmark("ss")
    return results


def rsgi_vs_asgi():
    results = {}
    with app("rsgi"):
        results["RSGI bytes"] = benchmark("b")
        results["RSGI str"] = benchmark("s")
    with app("asgi"):
        results["ASGI bytes"] = benchmark("b")
        results["ASGI str"] = benchmark("s")
    return results


def uvicorn():
    results = {}
    with app("asgi"):
        results["Granian ASGI"] = benchmark("b")
    with app("rsgi"):
        results["Granian RSGI"] = benchmark("b")
    with app("uvicorn_h11"):
        results["Uvicorn H11"] = benchmark("b")
    with app("uvicorn_httptools"):
        results["Uvicorn http-tools"] = benchmark("b")
    return results


def run():
    now = datetime.datetime.utcnow()
    results = {}
    # results["procs_threads"] = procs_threads()
    results["rsgi_body"] = rsgi_body_type()
    results["rsgi_asgi"] = rsgi_vs_asgi()
    results["uvicorn"] = uvicorn()
    with open(f"results/data.json", "w") as f:
        f.write(json.dumps({
            "cpu": CPU,
            "run_at": now.isoformat(),
            "results": results
        }))


if __name__ == "__main__":
    run()
