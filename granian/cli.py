from pathlib import Path
from typing import Optional

import typer

from .__version__ import __version__
from .constants import Interfaces, HTTPModes, ThreadModes
from .log import LogLevels
from .server import Granian


cli = typer.Typer(name="granian", context_settings={"ignore_unknown_options": True})


def version_callback(value: bool):
    if value:
        typer.echo(f"{cli.info.name} {__version__}")
        raise typer.Exit()


@cli.command()
def main(
    app: str = typer.Argument(..., help="Application target to serve."),
    host: str = typer.Option("127.0.0.1", help="Host address to bind to."),
    port: int = typer.Option(8000, help="Port to bind to."),
    interface: Interfaces = typer.Option(
        Interfaces.RSGI.value,
        help="Application interface type."
    ),
    http: HTTPModes = typer.Option(
        HTTPModes.auto.value,
        help="HTTP version."
    ),
    websockets: bool = typer.Option(
        True,
        "--ws/--no-ws",
        help="Enable websockets handling",
        show_default="enabled"
    ),
    workers: int = typer.Option(1, min=1, help="Number of worker processes."),
    threads: Optional[int] = typer.Option(None, min=1, help="Number of threads."),
    threading_mode: ThreadModes = typer.Option(
        ThreadModes.runtime.value,
        help="Threading mode to use."
    ),
    backlog: int = typer.Option(
        1024,
        min=128,
        help="Maximum number of connections to hold in backlog."
    ),
    log_level: LogLevels = typer.Option(
        LogLevels.info.value,
        help="Log level",
        case_sensitive=False
    ),
    ssl_keyfile: Optional[Path] = typer.Option(
        None,
        help="SSL key file",
        exists=True,
        file_okay=True,
        dir_okay=False,
        readable=True
    ),
    ssl_certificate: Optional[Path] = typer.Option(
        None,
        help="SSL certificate file",
        exists=True,
        file_okay=True,
        dir_okay=False,
        readable=True
    ),
    _: Optional[bool] = typer.Option(
        None,
        "--version",
        callback=version_callback,
        is_eager=True,
        help="Shows the version and exit."
    )
):
    Granian(
        app,
        address=host,
        port=port,
        workers=workers,
        backlog=backlog,
        threads=threads,
        threading_mode=threading_mode,
        interface=interface,
        http=http,
        websockets=websockets,
        log_level=log_level,
        ssl_cert=ssl_certificate,
        ssl_key=ssl_keyfile
    ).serve()
