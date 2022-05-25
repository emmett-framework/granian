from typing import Optional

import typer

from .__version__ import __version__
from .constants import Interfaces, ThreadModes
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
        interface=interface
    ).serve()
