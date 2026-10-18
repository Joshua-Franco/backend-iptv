import logging
import logging.config

LEVEL_COLORS = {
    "DEBUG": "\033[36m",
    "INFO": "\033[32m",
    "WARNING": "\033[33m",
    "ERROR": "\033[31m",
    "CRITICAL": "\033[1;31m",
}
RESET = "\033[0m"
DIM = "\033[2m"
MAGENTA = "\033[35m"

NAME_LABELS = {
    "uvicorn": "",
    "uvicorn.error": "",
}

LOGGING_CONFIG = {
    "version": 1,
    "disable_existing_loggers": False,
    "formatters": {
        "colored": {
            "()": "config.logging_setup.ColoredFormatter",
            "format": "{asctime} {levelname:<8} {name} {message}",
            "style": "{",
            "datefmt": "%H:%M:%S",
        },
        "uvicorn.access": {
            "()": "uvicorn.logging.AccessFormatter",
            "format": '%(levelprefix)s %(client_addr)s "%(request_line)s" %(status_code)s',
            "style": "%",
            "use_colors": True,
        },
    },
    "handlers": {
        "console": {
            "class": "logging.StreamHandler",
            "formatter": "colored",
            "stream": "ext://sys.stdout",
        },
        "access": {
            "class": "logging.StreamHandler",
            "formatter": "uvicorn.access",
            "stream": "ext://sys.stdout",
        },
    },
    "loggers": {
        "": {
            "handlers": ["console"],
            "level": "INFO",
        },
        "uvicorn": {
            "handlers": ["console"],
            "level": "INFO",
            "propagate": False,
        },
        "uvicorn.error": {
            "handlers": ["console"],
            "level": "INFO",
            "propagate": False,
        },
        "uvicorn.access": {
            "handlers": ["access"],
            "level": "INFO",
            "propagate": False,
        },
        "fastapi": {
            "handlers": ["console"],
            "level": "INFO",
            "propagate": False,
        },
        "httpx": {
            "handlers": ["console"],
            "level": "WARNING",
            "propagate": False,
        },
        "httpcore": {
            "handlers": ["console"],
            "level": "WARNING",
            "propagate": False,
        },
        "sqlalchemy.engine": {
            "handlers": ["console"],
            "level": "WARNING",
            "propagate": False,
        },
    },
}


class ColoredFormatter(logging.Formatter):
    def format(self, record: logging.LogRecord) -> str:
        time = self.formatTime(record, self.datefmt)
        levelname = record.levelname
        color = LEVEL_COLORS.get(levelname, "")
        level = f"{color}{levelname:<8}{RESET}"
        label = NAME_LABELS.get(record.name, record.name)
        parts = [f"{DIM}{time}{RESET}", level]
        if label:
            parts.append(f"{MAGENTA}{label}{RESET}")
        parts.append(record.getMessage())
        line = " ".join(parts)
        if record.exc_info and record.exc_info[0] is not None:
            line += "\n" + self.formatException(record.exc_info)
        return line


def setup_logging() -> None:
    logging.config.dictConfig(LOGGING_CONFIG)
