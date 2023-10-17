import json
from pathlib import Path
from typing import Optional

import typer

from .__version__ import __version__
from .constants import HTTPModes, Interfaces, Loops, ThreadModes
from .log import LogLevels
from .server import Granian


cli = typer.Typer(name='granian', context_settings={'auto_envvar_prefix': 'GRANIAN', 'ignore_unknown_options': True})


def version_callback(value: bool):
    if value:
        typer.echo(f'{cli.info.name} {__version__}')
        raise typer.Exit()


@cli.command()
def main(
    app: str = typer.Argument(..., help='Application target to serve.'),
    host: str = typer.Option('127.0.0.1', help='Host address to bind to.'),
    port: int = typer.Option(8000, help='Port to bind to.'),
    interface: Interfaces = typer.Option(Interfaces.RSGI.value, help='Application interface type.'),
    http: HTTPModes = typer.Option(HTTPModes.auto.value, help='HTTP version.'),
    websockets: bool = typer.Option(True, '--ws/--no-ws', help='Enable websockets handling', show_default='enabled'),
    workers: int = typer.Option(1, min=1, help='Number of worker processes.'),
    threads: int = typer.Option(1, min=1, help='Number of threads.'),
    threading_mode: ThreadModes = typer.Option(ThreadModes.workers.value, help='Threading mode to use.'),
    loop: Loops = typer.Option(Loops.auto.value, help='Event loop implementation'),
    loop_opt: bool = typer.Option(False, '--opt/--no-opt', help='Enable loop optimizations', show_default='disabled'),
    backlog: int = typer.Option(1024, min=128, help='Maximum number of connections to hold in backlog.'),
    log_enabled: bool = typer.Option(True, '--log/--no-log', help='Enable logging', show_default='enabled'),
    log_level: LogLevels = typer.Option(LogLevels.info.value, help='Log level', case_sensitive=False),
    log_config: Optional[Path] = typer.Option(
        None, help='Logging configuration file (json)', exists=True, file_okay=True, dir_okay=False, readable=True
    ),
    ssl_keyfile: Optional[Path] = typer.Option(
        None, help='SSL key file', exists=True, file_okay=True, dir_okay=False, readable=True
    ),
    ssl_certificate: Optional[Path] = typer.Option(
        None, help='SSL certificate file', exists=True, file_okay=True, dir_okay=False, readable=True
    ),
    url_path_prefix: Optional[str] = typer.Option(None, help='URL path prefix the app is mounted on'),
    reload: bool = typer.Option(
        False, '--reload/--no-reload', help="Enable auto reload on application's files changes"
    ),
    _: Optional[bool] = typer.Option(
        None,
        '--version',
        callback=version_callback,
        is_eager=True,
        help='Shows the version and exit.',
        allow_from_autoenv=False,
    ),
):
    log_dictconfig = None
    if log_config:
        with log_config.open() as log_config_file:
            try:
                log_dictconfig = json.loads(log_config_file.read())
            except Exception:
                print('Unable to parse provided logging config.')
                raise typer.Exit(1)

    Granian(
        app,
        address=host,
        port=port,
        interface=interface,
        workers=workers,
        threads=threads,
        pthreads=threads,
        threading_mode=threading_mode,
        loop=loop,
        loop_opt=loop_opt,
        http=http,
        websockets=websockets,
        backlog=backlog,
        log_enabled=log_enabled,
        log_level=log_level,
        log_dictconfig=log_dictconfig,
        ssl_cert=ssl_certificate,
        ssl_key=ssl_keyfile,
        url_path_prefix=url_path_prefix,
        reload=reload,
    ).serve()
