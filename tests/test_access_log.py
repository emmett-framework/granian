import logging
import time
from unittest.mock import MagicMock

from granian.asgi import _build_access_logger as asgi_build_access_logger
from granian.log import log_request_builder
from granian.rsgi import _build_access_logger as rsgi_build_access_logger
from granian.wsgi import _build_access_logger as wsgi_build_access_logger


_FMT = '%(method)s %(path)s %(status)d %(user_agent)s'


def _log_message(caplog):
    assert len(caplog.records) == 1
    return caplog.records[0].getMessage()


def _base_req(user_agent='TestAgent/1.0'):
    return {
        'addr_remote': '127.0.0.1',
        'protocol': 'HTTP/1.1',
        'path': '/test',
        'qs': '',
        'method': 'GET',
        'scheme': 'http',
        'user_agent': user_agent,
    }


def _asgi_scope(user_agent=b'TestAgent/1.0'):
    headers = [(b'user-agent', user_agent)] if user_agent is not None else []
    return {
        'client': ('127.0.0.1', 9999),
        'http_version': '1.1',
        'path': '/test',
        'query_string': '',
        'method': 'GET',
        'scheme': 'http',
        'headers': headers,
    }


def _rsgi_scope(user_agent='TestAgent/1.0'):
    scope = MagicMock()
    scope.client = '127.0.0.1:9999'
    scope.http_version = '1.1'
    scope.path = '/test'
    scope.query_string = ''
    scope.method = 'GET'
    scope.scheme = 'http'
    scope.headers.get.return_value = user_agent
    return scope


def _wsgi_scope(user_agent='TestAgent/1.0'):
    scope = {
        'REMOTE_ADDR': '127.0.0.1',
        'SERVER_PROTOCOL': 'HTTP/1.1',
        'PATH_INFO': '/test',
        'QUERY_STRING': '',
        'REQUEST_METHOD': 'GET',
        'wsgi.url_scheme': 'http',
    }
    if user_agent is not None:
        scope['HTTP_USER_AGENT'] = user_agent
    return scope


# ---------------------------------------------------------------------------
# log_request_builder (granian/log.py)
# ---------------------------------------------------------------------------


def test_log_request_user_agent(caplog):
    fn = log_request_builder(_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        fn(time.time(), time.perf_counter(), _base_req('Mozilla/5.0'), 200)
    assert 'Mozilla/5.0' in _log_message(caplog)


def test_log_request_missing_user_agent_dash(caplog):
    fn = log_request_builder(_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        fn(time.time(), time.perf_counter(), _base_req(user_agent='-'), 200)
    assert _log_message(caplog).endswith(' -')


# ---------------------------------------------------------------------------
# ASGI access logger (granian/asgi.py)
# ---------------------------------------------------------------------------


def test_asgi_access_log_user_agent(caplog):
    access_log, _ = asgi_build_access_logger(_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), _asgi_scope(b'curl/7.88'), 200)
    assert 'curl/7.88' in _log_message(caplog)


def test_asgi_access_log_absent_user_agent_defaults_to_dash(caplog):
    access_log, _ = asgi_build_access_logger(_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), _asgi_scope(user_agent=None), 200)
    assert _log_message(caplog).endswith(' -')


def test_asgi_access_log_user_agent_latin1_decoded(caplog):
    access_log, _ = asgi_build_access_logger(_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), _asgi_scope(b'Agent/\xe9'), 200)
    assert 'Agent/\xe9' in _log_message(caplog)


# ---------------------------------------------------------------------------
# RSGI access logger (granian/rsgi.py)
# ---------------------------------------------------------------------------


def test_rsgi_access_log_user_agent(caplog):
    access_log, _ = rsgi_build_access_logger(_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), _rsgi_scope('Go-http-client/2.0'), 200)
    assert 'Go-http-client/2.0' in _log_message(caplog)


def test_rsgi_access_log_absent_user_agent_defaults_to_dash(caplog):
    access_log, _ = rsgi_build_access_logger(_FMT)
    scope = _rsgi_scope()
    scope.headers.get.return_value = None
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), scope, 200)
    assert _log_message(caplog).endswith(' -')


# ---------------------------------------------------------------------------
# WSGI access logger (granian/wsgi.py)
# ---------------------------------------------------------------------------


def test_wsgi_access_log_user_agent(caplog):
    access_log, _ = wsgi_build_access_logger(_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), _wsgi_scope('python-httpx/0.27'), 200)
    assert 'python-httpx/0.27' in _log_message(caplog)


def test_wsgi_access_log_absent_user_agent_defaults_to_dash(caplog):
    access_log, _ = wsgi_build_access_logger(_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), _wsgi_scope(user_agent=None), 200)
    assert _log_message(caplog).endswith(' -')


# ---------------------------------------------------------------------------
# Arbitrary request header atoms: %({Header-Name}i)s
# ---------------------------------------------------------------------------

_HEADER_FMT = '%(method)s %(path)s %(status)d %({X-Request-Id}i)s %({x-forwarded-for}i)s'


def test_log_request_arbitrary_header(caplog):
    fn = log_request_builder(_HEADER_FMT)
    req = _base_req()
    req['get_header'] = {'x-request-id': 'abc-123', 'x-forwarded-for': '10.0.0.1'}.get
    with caplog.at_level(logging.INFO, logger='granian.access'):
        fn(time.time(), time.perf_counter(), req, 200)
    msg = _log_message(caplog)
    assert 'abc-123' in msg
    assert '10.0.0.1' in msg


def test_log_request_absent_arbitrary_header_defaults_to_dash(caplog):
    fn = log_request_builder(_HEADER_FMT)
    req = _base_req()
    req['get_header'] = {}.get
    with caplog.at_level(logging.INFO, logger='granian.access'):
        fn(time.time(), time.perf_counter(), req, 200)
    assert _log_message(caplog).endswith('- -')


def test_log_request_header_name_case_insensitive(caplog):
    # %({X-Request-Id}i)s and %({x-request-id}i)s should both key on 'x-request-id'
    for atom in ('%({X-Request-Id}i)s', '%({x-request-id}i)s', '%({X-REQUEST-ID}i)s'):
        fn = log_request_builder(atom)
        req = _base_req()
        req['get_header'] = {'x-request-id': 'test-id'}.get
        with caplog.at_level(logging.INFO, logger='granian.access'):
            fn(time.time(), time.perf_counter(), req, 200)
        assert 'test-id' in caplog.records[-1].getMessage()


def test_asgi_access_log_arbitrary_header(caplog):
    access_log, _ = asgi_build_access_logger(_HEADER_FMT)
    scope = _asgi_scope()
    scope['headers'] = [
        (b'user-agent', b'test'),
        (b'x-request-id', b'req-456'),
        (b'x-forwarded-for', b'192.168.1.1'),
    ]
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), scope, 200)
    msg = _log_message(caplog)
    assert 'req-456' in msg
    assert '192.168.1.1' in msg


def test_asgi_access_log_absent_arbitrary_header_defaults_to_dash(caplog):
    access_log, _ = asgi_build_access_logger(_HEADER_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), _asgi_scope(), 200)
    assert _log_message(caplog).endswith('- -')


def test_rsgi_access_log_arbitrary_header(caplog):
    access_log, _ = rsgi_build_access_logger(_HEADER_FMT)
    scope = _rsgi_scope()
    scope.headers.get.side_effect = lambda name: {
        'user-agent': 'test',
        'x-request-id': 'rsgi-789',
        'x-forwarded-for': '172.16.0.1',
    }.get(name)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), scope, 200)
    msg = _log_message(caplog)
    assert 'rsgi-789' in msg
    assert '172.16.0.1' in msg


def test_wsgi_access_log_arbitrary_header(caplog):
    access_log, _ = wsgi_build_access_logger(_HEADER_FMT)
    scope = _wsgi_scope()
    scope['HTTP_X_REQUEST_ID'] = 'wsgi-321'
    scope['HTTP_X_FORWARDED_FOR'] = '10.10.0.1'
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), scope, 200)
    msg = _log_message(caplog)
    assert 'wsgi-321' in msg
    assert '10.10.0.1' in msg


def test_wsgi_access_log_absent_arbitrary_header_defaults_to_dash(caplog):
    access_log, _ = wsgi_build_access_logger(_HEADER_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), _wsgi_scope(), 200)
    assert _log_message(caplog).endswith('- -')


# ---------------------------------------------------------------------------
# Arbitrary response header atoms: %({Header-Name}o)s
# ---------------------------------------------------------------------------

_RESP_HEADER_FMT = '%(method)s %(path)s %(status)d %({X-Trace-Id}o)s %({Content-Type}o)s'


def test_log_request_response_header(caplog):
    fn = log_request_builder(_RESP_HEADER_FMT)
    req = _base_req()
    req['get_response_header'] = {'x-trace-id': 'trace-abc', 'content-type': 'application/json'}.get
    with caplog.at_level(logging.INFO, logger='granian.access'):
        fn(time.time(), time.perf_counter(), req, 200)
    msg = _log_message(caplog)
    assert 'trace-abc' in msg
    assert 'application/json' in msg


def test_log_request_absent_response_header_defaults_to_dash(caplog):
    fn = log_request_builder(_RESP_HEADER_FMT)
    req = _base_req()
    req['get_response_header'] = {}.get
    with caplog.at_level(logging.INFO, logger='granian.access'):
        fn(time.time(), time.perf_counter(), req, 200)
    assert _log_message(caplog).endswith('- -')


def test_log_request_response_header_name_case_insensitive(caplog):
    for atom in ('%({X-Trace-Id}o)s', '%({x-trace-id}o)s', '%({X-TRACE-ID}o)s'):
        fn = log_request_builder(atom)
        req = _base_req()
        req['get_response_header'] = {'x-trace-id': 'tid-1'}.get
        with caplog.at_level(logging.INFO, logger='granian.access'):
            fn(time.time(), time.perf_counter(), req, 200)
        assert 'tid-1' in caplog.records[-1].getMessage()


def test_asgi_access_log_response_header(caplog):
    access_log, _ = asgi_build_access_logger(_RESP_HEADER_FMT)
    scope = _asgi_scope()
    resp_headers = [(b'x-trace-id', b'asgi-trace'), (b'content-type', b'text/plain')]
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), scope, 200, resp_headers)
    msg = _log_message(caplog)
    assert 'asgi-trace' in msg
    assert 'text/plain' in msg


def test_asgi_access_log_absent_response_header_defaults_to_dash(caplog):
    access_log, _ = asgi_build_access_logger(_RESP_HEADER_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), _asgi_scope(), 200, ())
    assert _log_message(caplog).endswith('- -')


def test_rsgi_access_log_response_header(caplog):
    access_log, _ = rsgi_build_access_logger(_RESP_HEADER_FMT)
    scope = _rsgi_scope()
    resp_headers = [('x-trace-id', 'rsgi-trace'), ('content-type', 'application/json')]
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), scope, 200, resp_headers)
    msg = _log_message(caplog)
    assert 'rsgi-trace' in msg
    assert 'application/json' in msg


def test_rsgi_access_log_absent_response_header_defaults_to_dash(caplog):
    access_log, _ = rsgi_build_access_logger(_RESP_HEADER_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), _rsgi_scope(), 200, ())
    assert _log_message(caplog).endswith('- -')


def test_wsgi_access_log_response_header(caplog):
    access_log, _ = wsgi_build_access_logger(_RESP_HEADER_FMT)
    resp_headers = [('X-Trace-Id', 'wsgi-trace'), ('Content-Type', 'text/html')]
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), _wsgi_scope(), 200, resp_headers)
    msg = _log_message(caplog)
    assert 'wsgi-trace' in msg
    assert 'text/html' in msg


def test_wsgi_access_log_absent_response_header_defaults_to_dash(caplog):
    access_log, _ = wsgi_build_access_logger(_RESP_HEADER_FMT)
    with caplog.at_level(logging.INFO, logger='granian.access'):
        access_log(time.time(), time.perf_counter(), _wsgi_scope(), 200, ())
    assert _log_message(caplog).endswith('- -')
