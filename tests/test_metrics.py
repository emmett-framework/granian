import asyncio
import re

import httpx
import pytest


N_REQUESTS = 10

HIST = 'granian_request_duration_seconds'
# Bucket upper bounds (seconds), plus `+Inf`.
EXPECTED_LE = [
    '0.005',
    '0.01',
    '0.025',
    '0.05',
    '0.075',
    '0.1',
    '0.25',
    '0.5',
    '0.75',
    '1',
    '2.5',
    '5',
    '7.5',
    '10',
    '+Inf',
]

_SAMPLE_RE = re.compile(r'^(?P<name>[^{\s]+)(?:\{(?P<labels>[^}]*)\})?\s+(?P<value>\S+)$')
_LABEL_RE = re.compile(r'(\w+)="([^"]*)"')


def _parse_metrics(text):
    """Parse Prometheus text exposition into {metric_name: [(labels_dict, value), ...]}."""
    out = {}
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        match = _SAMPLE_RE.match(line)
        labels = dict(_LABEL_RE.findall(match['labels'])) if match['labels'] else {}
        out.setdefault(match['name'], []).append((labels, float(match['value'])))
    return out


def _series_sum(metrics, name):
    """Sum every sample for `name`, across worker labels."""
    return sum(v for _, v in metrics.get(name, []))


def _hist_count(metrics):
    return _series_sum(metrics, f'{HIST}_count')


def _hist_bucket_inf(metrics):
    """Sum of the `+Inf` bucket across workers."""
    return sum(v for labels, v in metrics.get(f'{HIST}_bucket', []) if labels.get('le') == '+Inf')


async def _scrape(metrics_port):
    async with httpx.AsyncClient() as client:
        res = await client.get(f'http://127.0.0.1:{metrics_port}/', timeout=2)
    return res.text


async def _wait_for(metrics_port, predicate, timeout=20):
    """Scrape until `predicate(parsed_metrics)` holds (or timeout), return last body."""
    deadline = asyncio.get_running_loop().time() + timeout
    text = ''
    while asyncio.get_running_loop().time() < deadline:
        try:
            text = await _scrape(metrics_port)
        except Exception:
            await asyncio.sleep(0.5)
            continue
        if predicate(_parse_metrics(text)):
            return text
        await asyncio.sleep(0.5)
    return text


async def _wait_for_count(metrics_port, expected, timeout=20):
    return await _wait_for(metrics_port, lambda m: _hist_count(m) >= expected, timeout)


@pytest.mark.asyncio
@pytest.mark.parametrize('server_metrics', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_request_latency_histogram(server_metrics, runtime_mode):
    metrics_port, server = server_metrics

    async with server(runtime_mode, ws=False) as port:
        for _ in range(N_REQUESTS):
            res = httpx.get(f'http://localhost:{port}/info')
            assert res.status_code == 200

        text = await _wait_for_count(metrics_port, N_REQUESTS)

    metrics = _parse_metrics(text)

    assert f'{HIST}_bucket' in metrics
    assert f'{HIST}_sum' in metrics
    assert f'{HIST}_count' in metrics
    assert f'# TYPE {HIST} histogram' in text

    assert _hist_count(metrics) == N_REQUESTS
    assert _hist_bucket_inf(metrics) == N_REQUESTS
    assert _series_sum(metrics, f'{HIST}_sum') > 0.0
    # No static files mounted, so every request is dynamic.
    assert _series_sum(metrics, 'granian_requests_handled') == N_REQUESTS

    workers = {labels['worker'] for labels, _ in metrics[f'{HIST}_bucket']}
    assert workers

    for worker in workers:
        buckets = [(labels['le'], v) for labels, v in metrics[f'{HIST}_bucket'] if labels['worker'] == worker]
        by_le = dict(buckets)

        # Every bucket boundary present exactly once.
        assert sorted(by_le.keys(), key=lambda le: float('inf') if le == '+Inf' else float(le)) == EXPECTED_LE

        # Cumulative buckets are monotonically non-decreasing.
        ordered = [by_le[le] for le in EXPECTED_LE]
        assert all(ordered[i] <= ordered[i + 1] for i in range(len(ordered) - 1))

        count = next(v for labels, v in metrics[f'{HIST}_count'] if labels['worker'] == worker)
        assert by_le['+Inf'] == count


@pytest.mark.asyncio
@pytest.mark.parametrize('server_metrics_static', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['st'])
async def test_request_latency_histogram_excludes_static(server_metrics_static, runtime_mode):
    metrics_port, server = server_metrics_static

    async with server(runtime_mode, ws=False) as port:
        # One dynamic request (recorded) + several static ones (excluded).
        assert httpx.get(f'http://localhost:{port}/info').status_code == 200
        for _ in range(3):
            assert httpx.get(f'http://localhost:{port}/static/media.png').status_code == 200

        # Wait until both the dynamic and static requests are reflected.
        text = await _wait_for(
            metrics_port,
            lambda m: _hist_count(m) >= 1 and _series_sum(m, 'granian_static_requests_handled') >= 3,
        )

    metrics = _parse_metrics(text)

    # Only the dynamic request contributes to the histogram; static requests
    # are counted by their dedicated counters instead.
    assert _hist_count(metrics) == 1
    assert _hist_bucket_inf(metrics) == 1
    assert _series_sum(metrics, 'granian_static_requests_handled') == 3
    assert _series_sum(metrics, 'granian_requests_handled') == 4


@pytest.mark.asyncio
@pytest.mark.parametrize('server_metrics', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['st'])
async def test_request_latency_histogram_streaming_e2e(server_metrics, runtime_mode):
    metrics_port, server = server_metrics

    async with server(runtime_mode, ws=False) as port:
        # `/stream_slow` sends its body over ~0.3s. The recorded duration must
        # reflect the full send time, not time-to-first-byte.
        res = httpx.get(f'http://localhost:{port}/stream_slow')
        assert res.status_code == 200
        assert res.text == 'firstlast'

        text = await _wait_for_count(metrics_port, 1)

    metrics = _parse_metrics(text)

    assert _hist_count(metrics) == 1
    assert _hist_bucket_inf(metrics) == 1
    # The observation lands above the 0.1s boundary: proof of end-to-end timing.
    # A time-to-first-byte measurement would fall into a near-zero bucket.
    below_100ms = sum(v for labels, v in metrics[f'{HIST}_bucket'] if labels.get('le') == '0.1')
    assert below_100ms == 0
    assert _series_sum(metrics, f'{HIST}_sum') >= 0.25


@pytest.mark.asyncio
@pytest.mark.parametrize('server_metrics', ['asgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['st'])
async def test_request_latency_histogram_empty(server_metrics, runtime_mode):
    metrics_port, server = server_metrics

    async with server(runtime_mode, ws=False):
        # At least one scrape cycle without issuing requests.
        await asyncio.sleep(2)
        text = await _scrape(metrics_port)

    metrics = _parse_metrics(text)

    # Histogram is present with a zero count when nothing was observed.
    assert f'# TYPE {HIST} histogram' in text
    assert _hist_count(metrics) == 0
    assert _hist_bucket_inf(metrics) == 0
