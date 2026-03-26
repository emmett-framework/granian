import copy
import datetime
import logging
import logging.config
import re
import time
from enum import Enum
from typing import Any


class LogLevels(str, Enum):
    critical = 'critical'
    error = 'error'
    warning = 'warning'
    warn = 'warn'
    info = 'info'
    debug = 'debug'
    notset = 'notset'


log_levels_map = {
    LogLevels.critical: logging.CRITICAL,
    LogLevels.error: logging.ERROR,
    LogLevels.warning: logging.WARNING,
    LogLevels.warn: logging.WARN,
    LogLevels.info: logging.INFO,
    LogLevels.debug: logging.DEBUG,
    LogLevels.notset: logging.NOTSET,
}

LOGGING_CONFIG = {
    'version': 1,
    'disable_existing_loggers': False,
    'formatters': {
        'generic': {
            '()': 'logging.Formatter',
            'fmt': '[%(levelname)s] %(message)s',
            'datefmt': '[%Y-%m-%d %H:%M:%S %z]',
        },
        'access': {
            '()': 'logging.Formatter',
            'fmt': '%(message)s',
            'datefmt': '[%Y-%m-%d %H:%M:%S %z]',
        },
    },
    'handlers': {
        'console': {'formatter': 'generic', 'class': 'logging.StreamHandler', 'stream': 'ext://sys.stdout'},
        'access': {'formatter': 'access', 'class': 'logging.StreamHandler', 'stream': 'ext://sys.stdout'},
    },
    'loggers': {
        '_granian': {'handlers': ['console'], 'level': 'INFO', 'propagate': False},
        'granian.access': {'handlers': ['access'], 'level': 'INFO', 'propagate': False},
    },
}

DEFAULT_ACCESSLOG_FMT = '[%(time)s] %(addr)s - "%(method)s %(path)s %(protocol)s" %(status)d %(dt_ms).3f'

# NOTE: to be consistent with the Rust module logger name
logger = logging.getLogger('_granian')
access_logger = logging.getLogger('granian.access')


def configure_logging(level: LogLevels, config: dict[str, Any] | None = None, enabled: bool = True):
    log_config = copy.deepcopy(LOGGING_CONFIG)

    if config:
        log_config.update(config)

    log_config['loggers'].setdefault('_granian', {})['level'] = log_levels_map[level]
    logging.config.dictConfig(log_config)

    if not enabled:
        logger.setLevel(logging.CRITICAL + 1)


_REQUEST_HEADER_RE = re.compile(r'%\(\{([^}]+)\}i\)s')
_RESPONSE_HEADER_RE = re.compile(r'%\(\{([^}]+)\}o\)s')


def log_request_builder(fmt):
    required_req_headers = []
    required_resp_headers = []

    def _replace_req(m):
        name = m.group(1).strip().lower()
        required_req_headers.append(name)
        return f'%(header:{name})s'

    def _replace_resp(m):
        name = m.group(1).strip().lower()
        required_resp_headers.append(name)
        return f'%(resp_header:{name})s'

    transformed_fmt = _REQUEST_HEADER_RE.sub(_replace_req, fmt)
    transformed_fmt = _RESPONSE_HEADER_RE.sub(_replace_resp, transformed_fmt)

    # Validate the format string eagerly so misconfiguration is caught at startup,
    # not silently swallowed on the first real request.
    _dummy = {
        'addr': '127.0.0.1',
        'time': '2000-01-01 00:00:00 +0000',
        'dt_ms': 0.0,
        'status': 200,
        'path': '/',
        'query_string': '',
        'method': 'GET',
        'scheme': 'http',
        'protocol': 'HTTP/1.1',
        'user_agent': '-',
        **{f'header:{h}': '-' for h in required_req_headers},
        **{f'resp_header:{h}': '-' for h in required_resp_headers},
    }
    try:
        transformed_fmt % _dummy
    except (KeyError, ValueError) as exc:
        raise ValueError(f'Invalid access log format: {exc}') from exc

    now = datetime.datetime.now()
    local_now = now.astimezone()
    local_tz = local_now.tzinfo

    def log_request(rtime, mtime, req, res_code):
        dt = time.perf_counter() - mtime
        rdt = datetime.datetime.fromtimestamp(rtime, tz=local_tz)
        log_dict = {
            'addr': req['addr_remote'],
            'time': rdt.strftime('%Y-%m-%d %H:%M:%S %z'),
            'dt_ms': dt * 1000,
            'status': res_code,
            'path': req['path'],
            'query_string': req['qs'],
            'method': req['method'],
            'scheme': req['scheme'],
            'protocol': req['protocol'],
            'user_agent': req['user_agent'],
        }
        if required_req_headers:
            get_header = req.get('get_header') or (lambda _: None)
            for hname in required_req_headers:
                log_dict[f'header:{hname}'] = get_header(hname) or '-'
        if required_resp_headers:
            get_resp_header = req.get('get_response_header') or (lambda _: None)
            for hname in required_resp_headers:
                log_dict[f'resp_header:{hname}'] = get_resp_header(hname) or '-'
        access_logger.info(transformed_fmt, log_dict)

    return log_request
