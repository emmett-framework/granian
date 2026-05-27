"""Tests for the ASGI TLS extension (scope["extensions"]["tls"]).

https://asgi.readthedocs.io/en/latest/specs/tls.html

Covers presence/absence, the populated TLS metadata, mTLS client-certificate
delivery (single cert and full chains), PEM integrity, and — critically for the
trust boundary — that invalid or untrusted client certificates never reach the
application: they are rejected during the TLS handshake.
"""

import asyncio
import json
import pathlib
import ssl

import httpx
import pytest
import websockets


CERTS = pathlib.Path.cwd() / 'tests' / 'fixtures' / 'tls'

# Exceptions a client may observe when the server aborts the TLS handshake
# (e.g. rejecting an untrusted client certificate). The connection never
# produces an HTTP response, so the application is never invoked.
HANDSHAKE_REJECTED = (httpx.HTTPError, ssl.SSLError, OSError)


def _client_ctx(cert=None, key=None, max_version=None):
    ctx = ssl.create_default_context()
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    if cert is not None:
        ctx.load_cert_chain(certfile=str(cert), keyfile=str(key))
    if max_version is not None:
        ctx.maximum_version = max_version
    return ctx


def _split_pem_certs(pem_bundle):
    marker = '-----END CERTIFICATE-----'
    return [f'{block.strip()}\n{marker}\n' for block in pem_bundle.split(marker) if 'BEGIN CERTIFICATE' in block]


def _der(pem):
    return ssl.PEM_cert_to_DER_cert(f'{pem.strip()}\n')


# --- presence / absence ------------------------------------------------------


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_extension_present_on_tls(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode, ws=False, tls=True) as port:
        res = httpx.get(f'https://localhost:{port}/info', verify=False)

    assert res.status_code == 200
    tls = res.json()['extensions']['tls']
    assert isinstance(tls['tls_version'], int)
    assert isinstance(tls['cipher_suite'], int)
    assert tls['client_cert_chain'] == []
    assert tls['client_cert_name'] is None
    assert tls['client_cert_error'] is None
    assert tls['server_cert'] is None


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_extension_absent_on_plaintext(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode, ws=False, tls=False) as port:
        res = httpx.get(f'http://localhost:{port}/info')

    assert res.status_code == 200
    assert 'tls' not in res.json()['extensions']


# --- negotiated parameters ---------------------------------------------------


@pytest.mark.asyncio
@pytest.mark.parametrize(
    'tls_input,max_version,expected',
    [
        ('tls1.2', ssl.TLSVersion.TLSv1_2, 0x0303),
        ('tls1.3', ssl.TLSVersion.TLSv1_3, 0x0304),
    ],
)
async def test_tls_version_value(asgi_server, tls_input, max_version, expected):
    async with asgi_server('mt', ws=False, tls=True, tls_proto=tls_input) as port:
        res = httpx.get(f'https://localhost:{port}/info', verify=_client_ctx(max_version=max_version))

    assert res.json()['extensions']['tls']['tls_version'] == expected


# --- mTLS client certificate delivery ----------------------------------------


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_mtls_single_cert(asgi_server, runtime_mode):
    ctx = _client_ctx(CERTS / 'clientcert.pem', CERTS / 'clientkey.pem')
    async with asgi_server(runtime_mode, ws=False, tls=True, mtls=True) as port:
        res = httpx.get(f'https://localhost:{port}/info', verify=ctx)

    chain = res.json()['extensions']['tls']['client_cert_chain']
    assert len(chain) == 1
    assert 'BEGIN CERTIFICATE' in chain[0]


@pytest.mark.asyncio
async def test_mtls_intermediate_chain_order_and_integrity(asgi_server):
    ctx = _client_ctx(CERTS / 'clientchaincert.pem', CERTS / 'clientchainkey.pem')
    async with asgi_server('mt', ws=False, tls=True, mtls=True) as port:
        res = httpx.get(f'https://localhost:{port}/info', verify=ctx)

    delivered = res.json()['extensions']['tls']['client_cert_chain']
    expected = _split_pem_certs((CERTS / 'clientchaincert.pem').read_text())
    # Leaf-first ordering and byte-exact DER are both verified by comparing the
    # delivered chain against the exact bundle the client presented.
    assert len(delivered) == 2
    assert [_der(p) for p in delivered] == [_der(p) for p in expected]


@pytest.mark.asyncio
async def test_mtls_pem_round_trips_to_original_der(asgi_server):
    ctx = _client_ctx(CERTS / 'clientcert.pem', CERTS / 'clientkey.pem')
    async with asgi_server('mt', ws=False, tls=True, mtls=True) as port:
        res = httpx.get(f'https://localhost:{port}/info', verify=ctx)

    delivered_pem = res.json()['extensions']['tls']['client_cert_chain'][0]
    original_der = _der((CERTS / 'clientcert.pem').read_text())
    assert _der(delivered_pem) == original_der


@pytest.mark.asyncio
async def test_mtls_chain_consistent_across_keepalive(asgi_server):
    ctx = _client_ctx(CERTS / 'clientcert.pem', CERTS / 'clientkey.pem')
    async with asgi_server('mt', ws=False, tls=True, mtls=True) as port:
        with httpx.Client(verify=ctx) as client:
            first = client.get(f'https://localhost:{port}/info').json()
            second = client.get(f'https://localhost:{port}/info').json()

    chain1 = first['extensions']['tls']['client_cert_chain']
    chain2 = second['extensions']['tls']['client_cert_chain']
    assert len(chain1) == 1
    assert chain1 == chain2


# --- trust boundary: invalid certs must NOT reach the application ------------


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_untrusted_client_cert_rejected(asgi_server, runtime_mode):
    # A cert signed by an untrusted CA, presented with mandatory verification.
    ctx = _client_ctx(CERTS / 'roguecert.pem', CERTS / 'roguekey.pem')
    async with asgi_server(runtime_mode, ws=False, tls=True, mtls=True) as port:
        with pytest.raises(HANDSHAKE_REJECTED):
            httpx.get(f'https://localhost:{port}/info', verify=ctx)


@pytest.mark.asyncio
async def test_missing_client_cert_rejected_when_required(asgi_server):
    async with asgi_server('mt', ws=False, tls=True, mtls=True) as port:
        with pytest.raises(HANDSHAKE_REJECTED):
            httpx.get(f'https://localhost:{port}/info', verify=False)


@pytest.mark.asyncio
async def test_untrusted_cert_rejected_even_when_verification_optional(asgi_server):
    # Key property: with a CA configured but client verification optional
    # (allow_unauthenticated), a *presented* certificate must still be valid.
    # Optional means the cert may be absent — never that an invalid cert is
    # accepted and surfaced to the app as if trusted.
    ctx = _client_ctx(CERTS / 'roguecert.pem', CERTS / 'roguekey.pem')
    async with asgi_server('mt', ws=False, tls=True, mtls='optional') as port:
        with pytest.raises(HANDSHAKE_REJECTED):
            httpx.get(f'https://localhost:{port}/info', verify=ctx)


@pytest.mark.asyncio
async def test_no_cert_allowed_when_verification_optional(asgi_server):
    async with asgi_server('mt', ws=False, tls=True, mtls='optional') as port:
        res = httpx.get(f'https://localhost:{port}/info', verify=False)

    assert res.status_code == 200
    assert res.json()['extensions']['tls']['client_cert_chain'] == []


# --- protocol coverage: HTTP/2 and WebSocket ---------------------------------


@pytest.mark.asyncio
async def test_h2_tls_extension_present(asgi_server):
    async with asgi_server('mt', ws=False, tls=True) as port:
        with httpx.Client(verify=False, http2=True) as client:
            res = client.get(f'https://localhost:{port}/info')

    assert res.http_version == 'HTTP/2'
    assert isinstance(res.json()['extensions']['tls']['tls_version'], int)


@pytest.mark.asyncio
async def test_h2_mtls_client_cert_present(asgi_server):
    ctx = _client_ctx(CERTS / 'clientcert.pem', CERTS / 'clientkey.pem')
    async with asgi_server('mt', ws=False, tls=True, mtls=True) as port:
        with httpx.Client(verify=ctx, http2=True) as client:
            res = client.get(f'https://localhost:{port}/info')

    assert res.http_version == 'HTTP/2'
    assert len(res.json()['extensions']['tls']['client_cert_chain']) == 1


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_ws_tls_extension_present(asgi_server, runtime_mode):
    ssl_context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    ssl_context.load_verify_locations(str(CERTS / 'cert.pem'))
    async with asgi_server(runtime_mode, tls=True) as port:
        async with websockets.connect(f'wss://localhost:{port}/ws_info', ssl=ssl_context) as ws:
            data = json.loads(await ws.recv())

    assert 'tls' in data['extensions']
    assert isinstance(data['extensions']['tls']['tls_version'], int)
    assert data['extensions']['tls']['client_cert_chain'] == []


# --- certificate revocation (CRL) --------------------------------------------


@pytest.mark.asyncio
async def test_revoked_cert_accepted_without_crl(asgi_server):
    # Control: the revoked cert is otherwise valid (chains to the trusted CA),
    # so without a CRL configured it is accepted.
    ctx = _client_ctx(CERTS / 'revokedcert.pem', CERTS / 'revokedkey.pem')
    async with asgi_server('mt', ws=False, tls=True, mtls=True) as port:
        res = httpx.get(f'https://localhost:{port}/info', verify=ctx)

    assert res.status_code == 200
    assert len(res.json()['extensions']['tls']['client_cert_chain']) == 1


@pytest.mark.asyncio
async def test_revoked_cert_rejected_with_crl(asgi_server):
    # With the CRL configured, the same cert is rejected at the handshake and
    # never reaches the application.
    ctx = _client_ctx(CERTS / 'revokedcert.pem', CERTS / 'revokedkey.pem')
    async with asgi_server('mt', ws=False, tls=True, mtls=True, crl=True) as port:
        with pytest.raises(HANDSHAKE_REJECTED):
            httpx.get(f'https://localhost:{port}/info', verify=ctx)


# --- per-connection isolation under concurrency ------------------------------


@pytest.mark.asyncio
async def test_concurrent_distinct_clients_isolated(asgi_server):
    # Two clients presenting different certificates issue overlapping requests;
    # each response must carry that client's own chain (extraction is per
    # connection, so chains can never be crossed between connections).
    single = _client_ctx(CERTS / 'clientcert.pem', CERTS / 'clientkey.pem')
    chained = _client_ctx(CERTS / 'clientchaincert.pem', CERTS / 'clientchainkey.pem')

    async with asgi_server('mt', ws=False, tls=True, mtls=True) as port:
        url = f'https://localhost:{port}/info'
        async with httpx.AsyncClient(verify=single) as ca, httpx.AsyncClient(verify=chained) as cb:
            for _ in range(5):
                ra, rb = await asyncio.gather(ca.get(url), cb.get(url))
                assert len(ra.json()['extensions']['tls']['client_cert_chain']) == 1
                assert len(rb.json()['extensions']['tls']['client_cert_chain']) == 2
