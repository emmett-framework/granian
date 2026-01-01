from unittest.mock import patch

import click
import pytest

from granian import Granian
from granian.cli import Duration
from granian.errors import ConfigurationError


@pytest.mark.parametrize(
    ('value', 'expected'),
    (
        ('10', 10),
        ('10s', 10),
        ('10m', 60 * 10),
        ('10m10s', 60 * 10 + 10),
        ('10h', 60 * 60 * 10),
        ('10d', 24 * 60 * 60 * 10),
        ('10d10h10m10s', 24 * 60 * 60 * 10 + 60 * 60 * 10 + 60 * 10 + 10),
    ),
)
def test_duration_convert(value: str, expected: int) -> None:
    duration_type = Duration()
    assert duration_type.convert(value, None, None) == expected


@pytest.mark.parametrize(
    ('value', 'error_message'),
    (
        ('10x', r"'10x' is not a valid duration"),
        ('10d10h10m10s10', r"'10d10h10m10s10' is not a valid duration"),
        ('10d10h10m10s10', r"'10d10h10m10s10' is not a valid duration"),
    ),
)
def test_duration_convert_invalid(value: str, error_message: str) -> None:
    duration_type = Duration()
    with pytest.raises(click.BadParameter, match=error_message):
        duration_type.convert(value, None, None)


@pytest.mark.parametrize(
    ('value', 'error_message'),
    (
        ('1000', r"'1000' is greater than the maximum allowed value of 100 seconds"),
        ('30m', r"'30m' is greater than the maximum allowed value of 100 seconds"),
        ('5', r"'5' is less than the minimum allowed value of 10 seconds"),
    ),
)
def test_duration_convert_out_of_range(value: str, error_message: str) -> None:
    duration_type = Duration(10, 100)
    with pytest.raises(click.BadParameter, match=error_message):
        duration_type.convert(value, None, None)


def test_workers_lifetime_with_reload() -> None:
    """Test that workers_lifetime with reload=True doesn't raise TypeError.

    Regression test for when workers_lifetime was set to None before the < 60 check,
    causing a TypeError when comparing None < 60.
    """
    server = Granian('tests.apps.asgi:app', interface='asgi', reload=True, workers_lifetime=100)
    with patch.object(server, '_serve_loop'), patch.object(server, '_serve_with_reloader'):
        server.serve()
    assert server.workers_lifetime is None


def test_workers_lifetime_below_minimum() -> None:
    """Test that workers_lifetime below 60 raises ConfigurationError."""
    server = Granian('tests.apps.asgi:app', interface='asgi', workers_lifetime=30)
    with pytest.raises(ConfigurationError):
        server.serve()
