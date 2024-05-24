import copy
import datetime
import logging
import logging.config
import time
from enum import Enum
from typing import Any, Dict, Optional


class LogLevels(str, Enum):
    critical = 'critical'
    error = 'error'
    warning = 'warning'
    warn = 'warn'
    info = 'info'
    debug = 'debug'


log_levels_map = {
    LogLevels.critical: logging.CRITICAL,
    LogLevels.error: logging.ERROR,
    LogLevels.warning: logging.WARNING,
    LogLevels.warn: logging.WARN,
    LogLevels.info: logging.INFO,
    LogLevels.debug: logging.DEBUG,
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


def configure_logging(level: LogLevels, config: Optional[Dict[str, Any]] = None, enabled: bool = True):
    log_config = copy.deepcopy(LOGGING_CONFIG)

    if config:
        log_config.update(config)

    log_config['loggers']['_granian']['level'] = log_levels_map[level]
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
        access_logger.info(
            fmt,
            {
                'addr': req['addr_remote'],
                'time': rdt.strftime('%Y-%m-%d %H:%M:%S %z'),
                'dt_ms': dt * 1000,
                'status': res_code,
                'path': req['path'],
                'query_string': req['qs'],
                'method': req['method'],
                'scheme': req['scheme'],
                'protocol': req['protocol'],
            },
        )

    return log_request
