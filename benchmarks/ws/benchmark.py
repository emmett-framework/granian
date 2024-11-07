import asyncio
import json
import os
import time

import websockets


CONCURRENCY = int(os.environ.get('BENCHMARK_CONCURRENCY', '16'))
MSG_NO = int(os.environ.get('BENCHMARK_MSGNO', '2500'))
RUNS = int(os.environ.get('BENCHMARK_RUNS', '1'))
MEASUREMENTS = []

ready = asyncio.Event()


def _client_redy(target):
    acks = {'all': 0}

    def inner():
        acks['all'] += 1
        if acks['all'] == target:
            ready.set()

    return inner


async def _client_recv(ws, to_recv):
    t_start = time.time()
    recv = 0
    while recv < to_recv:
        await ws.recv()
        recv += 1
    return (recv, time.time() - t_start)


async def _client_send(ws, messages):
    t_start = time.time()
    sent = 0
    for message in messages:
        await ws.send(message)
        sent += 1
    return (sent, time.time() - t_start)


async def client(idx, messages, ready_signal):
    # print(f'Starting client {idx}..')
    async with websockets.connect('ws://127.0.0.1:8000') as ws:
        ready_signal()
        await ready.wait()
        # print(f'Client {idx} ready')
        await asyncio.sleep(3)

        _task_recv, _task_send = (
            asyncio.create_task(_client_recv(ws, len(messages) * CONCURRENCY)),
            asyncio.create_task(_client_send(ws, messages))
        )
        t_start = time.time()
        recv_data, send_data = await asyncio.gather(_task_recv, _task_send)
        t_end = time.time() - t_start
    # print(f'Client {idx} terminated')
    return (recv_data, send_data, t_end)


async def benchmark(run):
    # print(f'Starting benchmark run {run}..')
    messages = [f'msg{str(idx).zfill(len(str(MSG_NO)))}'.encode('utf8') for idx in range(MSG_NO)]
    client_ready = _client_redy(CONCURRENCY)
    tasks = []
    for idx in range(CONCURRENCY):
        tasks.append(asyncio.create_task(client(idx, messages, client_ready)))
    res = await asyncio.gather(*tasks)
    MEASUREMENTS.append(res)
    ready.clear()
    # print(f'Completed benchmark run {run}')


def build_results(data):
    rv = []
    for run in data:
        tot_timings = []
        recv_timings = []
        send_timings = []
        sum_timings = []
        tot_throughput = []
        recv_throughput = []
        send_throughput = []
        sum_throughput = []
        for recv_data, send_data, client_time in run:
            recv_timings.append(recv_data[1])
            send_timings.append(send_data[1])
            tot_timings.append(client_time)
            sum_timings.append(max(recv_data[1], send_data[1]))
            recv_throughput.append(MSG_NO * CONCURRENCY / recv_data[1])
            send_throughput.append(MSG_NO / send_data[1])
            tot_throughput.append(MSG_NO * (CONCURRENCY + 1) / client_time)
            sum_throughput.append(MSG_NO * (CONCURRENCY + 1) / sum_timings[-1])

        recv_avg, recv_max, recv_min = sum(recv_timings) / len(recv_timings), max(recv_timings), min(recv_timings)
        send_avg, send_max, send_min = sum(send_timings) / len(send_timings), max(send_timings), min(send_timings)
        tot_avg, tot_max, tot_min = sum(tot_timings) / len(tot_timings), max(tot_timings), min(tot_timings)
        sum_avg, sum_max, sum_min = sum(sum_timings) / len(sum_timings), max(sum_timings), min(sum_timings)
        th_recv, th_send, th_all, th_sum = sum(recv_throughput), sum(send_throughput), sum(tot_throughput), sum(sum_throughput)
        res = {
            'timings': {
                'recv': {'avg': recv_avg, 'max': recv_max, 'min': recv_min},
                'send': {'avg': send_avg, 'max': send_max, 'min': send_min},
                'sum': {'avg': sum_avg, 'max': sum_max, 'min': sum_min},
                'all': {'avg': tot_avg, 'max': tot_max, 'min': tot_min},
            },
            'throughput': {
                'recv': th_recv,
                'send': th_send,
                'all': th_all,
                'sum': th_sum,
            }
        }
        rv.append(res)
    return rv


async def main():
    for idx in range(RUNS):
        await benchmark(idx + 1)

    res = build_results(MEASUREMENTS)
    res.sort(key=lambda item: item['throughput']['sum'])
    ridx = 2 if len(res) > 1 else 1
    print(json.dumps(res[-ridx]))


if __name__ == '__main__':
    asyncio.run(main())
