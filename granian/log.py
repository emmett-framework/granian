import copy
import logging
import logging.config

from enum import Enum


class LogLevels(str, Enum):
    critical = "critical"
    error = "error"
    warning = "warning"
    warn = "warn"
    info = "info"
    debug = "debug"


log_levels_map = {
    LogLevels.critical: logging.CRITICAL,
    LogLevels.error: logging.ERROR,
    LogLevels.warning: logging.WARNING,
    LogLevels.warn: logging.WARN,
    LogLevels.info: logging.INFO,
    LogLevels.debug: logging.DEBUG
}

LOGGING_CONFIG = {
    "version": 1,
    "disable_existing_loggers": False,
    "root": {"level": "INFO", "handlers": ["console"]},
    "formatters": {
        "generic": {
            "()": "logging.Formatter",
            "fmt": "[%(levelname)s] %(message)s",
            "datefmt": "[%Y-%m-%d %H:%M:%S %z]"
        }
    },
    "handlers": {
        "console": {
            "formatter": "generic",
            "class": "logging.StreamHandler",
            "stream": "ext://sys.stdout",
        }
    }
}

logger = logging.getLogger()


def configure_logging(level: LogLevels):
    config = copy.deepcopy(LOGGING_CONFIG)
    config["root"]["level"] = log_levels_map[level]
    logging.config.dictConfig(config)
