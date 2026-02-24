import httpx
import pytest


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
@pytest.mark.parametrize('file_name', ['media.png', 'こんにちは.png'])
async def test_static_files(server_static_files, runtime_mode, file_name):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/static/{file_name}')

    assert res.status_code == 200
    assert res.headers.get('content-type') == 'image/png'
    assert res.headers.get('cache-control')


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_notfound(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/static/missing.png')

    assert res.status_code == 404


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_dir_no_rewrite(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/static/empty')

    assert res.status_code == 404


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_outsidemount(monkeypatch, server_static_files, runtime_mode):
    monkeypatch.setattr(httpx._urlparse, 'normalize_path', lambda v: v)
    # otherwise httpx will convert /static/../conftest.py to /conftest.py before request

    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/static/../conftest.py')

    assert res.status_code == 404


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_approute(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/info')

    assert res.status_code == 200


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_rewrite_index(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False, static_rewrite=True) as port:
        res = httpx.get(f'http://localhost:{port}/static/file_rewrite')

    assert res.status_code == 200


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_rewrite_notfound(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False, static_rewrite=True) as port:
        res = httpx.get(f'http://localhost:{port}/static/empty')

    assert res.status_code == 404


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_multi(server_static_files, runtime_mode):
    async with server_static_files(
        runtime_mode, ws=False, static_mount=[('/static', 'static'), ('/other', 'tls')]
    ) as port:
        res1 = httpx.get(f'http://localhost:{port}/static/media.png')
        res2 = httpx.get(f'http://localhost:{port}/other/cert.pem')

    assert res1.status_code == 200
    assert res2.status_code == 200
    assert res1.headers.get('content-type') == 'image/png'
    assert res2.headers.get('content-type') == 'application/x-x509-ca-cert'


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_multi_precompressed(server_static_files, runtime_mode):
    async with server_static_files(
        runtime_mode, ws=False, static_mount=[('/static', 'static'), ('/other', 'tls')], static_precompressed=True
    ) as port:
        # Precompressed from first mount
        headers = {'Accept-Encoding': 'br'}
        res1 = httpx.get(f'http://localhost:{port}/static/media.png', headers=headers)
        # Plain file from second mount (no sidecars exist)
        res2 = httpx.get(f'http://localhost:{port}/other/cert.pem', headers=headers)

    assert res1.status_code == 200
    assert res1.headers.get('content-type') == 'image/png'
    assert res1.headers.get('content-encoding') == 'br'
    assert res2.status_code == 200
    assert res2.headers.get('content-type') == 'application/x-x509-ca-cert'
    assert res2.headers.get('content-encoding') is None  # no sidecar, falls back to plain


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files_precompressed', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
@pytest.mark.parametrize(
    'accept_encoding,expected_encoding',
    [
        ('br', 'br'),
        ('gzip', 'gzip'),
        ('zstd', 'zstd'),
    ],
)
async def test_static_files_precompressed(
    server_static_files_precompressed, runtime_mode, accept_encoding, expected_encoding
):
    async with server_static_files_precompressed(runtime_mode, ws=False) as port:
        headers = {'Accept-Encoding': accept_encoding}
        res = httpx.get(f'http://localhost:{port}/static/media.png', headers=headers)

    assert res.status_code == 200
    assert res.headers.get('content-type') == 'image/png'
    assert res.headers.get('content-encoding') == expected_encoding
    assert res.headers.get('vary') == 'accept-encoding'


# only use one interface here since the negotiation logic in Rust is
# interface-agnostic and tests take long to run
@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files_precompressed', ['rsgi'], indirect=True)
@pytest.mark.parametrize(
    'accept_encoding,expected_encoding',
    [
        # multiple encodings - client order wins when q-values equal (first one wins)
        ('br, gzip', 'br'),
        ('gzip, br', 'gzip'),
        ('zstd, gzip', 'zstd'),
        ('gzip, zstd', 'gzip'),
        ('br, zstd', 'br'),
        ('zstd, br', 'zstd'),
        ('br, zstd, gzip', 'br'),
        # q-value priority - client preference respected
        ('gzip;q=1.0, br;q=0.5', 'gzip'),
        ('br;q=0.5, gzip;q=1.0', 'gzip'),
        ('zstd;q=0.5, br;q=1.0', 'br'),
        ('gzip;q=0.9, br;q=0.8, zstd;q=1.0', 'zstd'),
        ('gzip;q=1.0, br;q=0.9, zstd;q=0.8', 'gzip'),
        # q=0 means explicitly rejected
        ('gzip;q=0, br', 'br'),
        ('zstd;q=0, br;q=0, gzip', 'gzip'),
        # unsupported encodings
        ('deflate', None),
        ('identity', None),
    ],
)
async def test_static_files_precompressed_negotiation(
    server_static_files_precompressed, accept_encoding, expected_encoding
):
    async with server_static_files_precompressed('mt', ws=False) as port:
        headers = {'Accept-Encoding': accept_encoding}
        res = httpx.get(f'http://localhost:{port}/static/media.png', headers=headers)

    assert res.status_code == 200
    assert res.headers.get('content-type') == 'image/png'
    if expected_encoding:
        assert res.headers.get('content-encoding') == expected_encoding
        assert res.headers.get('vary') == 'accept-encoding'
    else:
        assert res.headers.get('content-encoding') is None


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files_precompressed', ['rsgi'], indirect=True)
async def test_static_files_precompressed_no_accept_encoding(server_static_files_precompressed):
    async with server_static_files_precompressed('mt', ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/static/media.png', headers={'Accept-Encoding': ''})

    assert res.status_code == 200
    assert res.headers.get('content-type') == 'image/png'
    assert res.headers.get('content-encoding') is None


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files_precompressed', ['rsgi'], indirect=True)
@pytest.mark.parametrize('accept_encoding', ['br', 'gzip', 'zstd', 'br, gzip, zstd'])
async def test_static_files_precompressed_fallback_to_plain(server_static_files_precompressed, accept_encoding):
    async with server_static_files_precompressed('mt', ws=False) as port:
        headers = {'Accept-Encoding': accept_encoding}
        res = httpx.get(f'http://localhost:{port}/static/こんにちは.png', headers=headers)
        # こんにちは.png has no compressed sidecars, so should always serve plain

    assert res.status_code == 200
    assert res.headers.get('content-type') == 'image/png'
    assert res.headers.get('content-encoding') is None


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files_precompressed', ['rsgi'], indirect=True)
@pytest.mark.parametrize(
    'accept_encoding,expected_encoding',
    [
        ('gzip;q=invalid', 'gzip'),  # invalid defaults to 1.0
        ('gzip;q=', 'gzip'),  # empty defaults to 1.0
        ('gzip;q=2.0', 'gzip'),  # q>1.0 clamped to 1.0
        ('gzip;q=-0.5', None),  # negative clamped to 0.0 (rejected)
        (',,,gzip,,,', 'gzip'),  # multiple commas handled
        # note: can't test leading/trailing whitespace ('  gzip  ') - httpx rejects it
    ],
)
async def test_static_files_precompressed_malformed_qvalues(
    server_static_files_precompressed, accept_encoding, expected_encoding
):
    async with server_static_files_precompressed('mt', ws=False) as port:
        headers = {'Accept-Encoding': accept_encoding}
        res = httpx.get(f'http://localhost:{port}/static/media.png', headers=headers)

    assert res.status_code == 200
    assert res.headers.get('content-type') == 'image/png'
    if expected_encoding:
        assert res.headers.get('content-encoding') == expected_encoding
    else:
        assert res.headers.get('content-encoding') is None


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files_precompressed', ['rsgi'], indirect=True)
@pytest.mark.parametrize(
    'accept_encoding,expected_encoding',
    [
        ('*', 'zstd'),  # matches best available (zstd > br > gzip)
        ('gzip, *', 'gzip'),  # explicit gzip before wildcard
        ('*;q=0.5, gzip;q=1.0', 'gzip'),  # q-value on wildcard
        ('zstd;q=0, *', 'br'),  # reject zstd, wildcard returns next best (br)
        ('zstd;q=0, br;q=0, *', 'gzip'),  # reject zstd and br, wildcard returns gzip
    ],
)
async def test_static_files_precompressed_wildcard(
    server_static_files_precompressed, accept_encoding, expected_encoding
):
    async with server_static_files_precompressed('mt', ws=False) as port:
        headers = {'Accept-Encoding': accept_encoding}
        res = httpx.get(f'http://localhost:{port}/static/media.png', headers=headers)

    assert res.status_code == 200
    assert res.headers.get('content-type') == 'image/png'
    assert res.headers.get('content-encoding') == expected_encoding
    assert res.headers.get('vary') == 'accept-encoding'


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files_precompressed', ['rsgi'], indirect=True)
async def test_static_files_precompressed_unicode_filename(server_static_files_precompressed):
    async with server_static_files_precompressed('mt', ws=False) as port:
        headers = {'Accept-Encoding': 'br, gzip, zstd'}
        res = httpx.get(f'http://localhost:{port}/static/こんにちは.png', headers=headers)

    assert res.status_code == 200
    assert res.headers.get('content-type') == 'image/png'
