import copy
import logging
import logging.config
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
        }
    },
    'handlers': {'console': {'formatter': 'generic', 'class': 'logging.StreamHandler', 'stream': 'ext://sys.stdout'}},
    'loggers': {'_granian': {'handlers': ['console'], 'level': 'INFO', 'propagate': False}},
}

# NOTE: to be consistent with the Rust module logger name
logger = logging.getLogger('_granian')


def configure_logging(level: LogLevels, config: Optional[Dict[str, Any]] = None, enabled: bool = True):
    log_config = copy.deepcopy(LOGGING_CONFIG)

    if config:
        log_config.update(config)

    log_config['loggers']['_granian']['level'] = log_levels_map[level]
    logging.config.dictConfig(log_config)

    if not enabled:
        logger.setLevel(logging.CRITICAL + 1)
