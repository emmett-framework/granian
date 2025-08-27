import copy
import datetime
import logging
import logging.config
import time
from enum import Enum
from typing import Any, Dict, Optional


class SafeAtoms(dict):
    def __init__(self, atoms):
        dict.__init__(self)
        for key, value in atoms.items():
            if isinstance(value, str):
                self[key] = value.replace('"', '\\"')
            else:
                self[key] = value

    def __getitem__(self, k):
        if k.startswith('{'):
            kl = k.lower()
            if kl in self:
                return super().__getitem__(kl)
            else:
                return '-'
        if k in self:
            return super().__getitem__(k)
        else:
            return '-'


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
        'console': {
            'formatter': 'generic',
            'class': 'logging.StreamHandler',
            'stream': 'ext://sys.stdout',
        },
        'access': {
            'formatter': 'access',
            'class': 'logging.StreamHandler',
            'stream': 'ext://sys.stdout',
        },
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


def configure_logging(level: LogLevels, config: Optional[Dict[str, Any]] = None, enabled: bool = True):
    log_config = copy.deepcopy(LOGGING_CONFIG)

    if config:
        log_config.update(config)

    log_config['loggers'].setdefault('_granian', {})['level'] = log_levels_map[level]
    logging.config.dictConfig(log_config)

    if not enabled:
        logger.setLevel(logging.CRITICAL + 1)


def log_request_builder(fmt):
    now = datetime.datetime.now()
    local_now = now.astimezone()
    local_tz = local_now.tzinfo

    def log_request(rtime, req, res_code):
        dt = time.time() - rtime
        rdt = datetime.datetime.fromtimestamp(rtime, tz=local_tz)
        atoms_context = {
            'addr': req['addr_remote'],
            'time': rdt.strftime('%Y-%m-%d %H:%M:%S %z'),
            'dt_ms': dt * 1000,
            'status': res_code,
            'path': req['path'],
            'query_string': req['qs'],
            'method': req['method'],
            'scheme': req['scheme'],
            'protocol': req['protocol'],
            # request body length
            'rbl': req['response_body_length'],
        }
        context = SafeAtoms(atoms_context)
        del req['addr_remote']
        del req['path']
        del req['qs']
        del req['method']
        del req['scheme']
        del req['protocol']
        del req['response_body_length']
        context.update(req)
        access_logger.info(
            fmt,
            context,
        )

    return log_request
