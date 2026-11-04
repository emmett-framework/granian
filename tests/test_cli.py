import click
import pytest

from granian.cli import Duration, cli


def test_ws_ping_options_exposed() -> None:
    params = {param.name: param for param in cli.params}
    assert params['ws_ping_interval'].default is None
    assert params['ws_ping_timeout'].default == 60.0
    assert params['ws_ping_interval'].show_envvar
    assert params['ws_ping_timeout'].show_envvar


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
