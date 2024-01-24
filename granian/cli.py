import json
from pathlib import Path
from typing import Optional

import typer

from . import __version__
from .constants import HTTPModes, Interfaces, Loops, ThreadModes
from .http import HTTP1Settings, HTTP2Settings
from .log import LogLevels
from .server import Granian


cli = typer.Typer(name='granian', context_settings={'auto_envvar_prefix': 'GRANIAN', 'ignore_unknown_options': True})


def version_callback(value: bool):
    if value:
        typer.echo(f'{cli.info.name} {__version__}')
        raise typer.Exit()


@cli.command()
def main(
    app: str = typer.Argument(..., help='Application target to serve'),
    host: str = typer.Option('127.0.0.1', help='Host address to bind to'),
    port: int = typer.Option(8000, help='Port to bind to.'),
    interface: Interfaces = typer.Option(Interfaces.RSGI.value, help='Application interface type'),
    http: HTTPModes = typer.Option(HTTPModes.auto.value, help='HTTP version'),
    websockets: bool = typer.Option(True, '--ws/--no-ws', help='Enable websockets handling', show_default='enabled'),
    workers: int = typer.Option(1, min=1, help='Number of worker processes'),
    threads: int = typer.Option(1, min=1, help='Number of threads'),
    blocking_threads: int = typer.Option(1, min=1, help='Number of blocking threads'),
    threading_mode: ThreadModes = typer.Option(ThreadModes.workers.value, help='Threading mode to use'),
    loop: Loops = typer.Option(Loops.auto.value, help='Event loop implementation'),
    loop_opt: bool = typer.Option(False, '--opt/--no-opt', help='Enable loop optimizations', show_default='disabled'),
    backlog: int = typer.Option(1024, min=128, help='Maximum number of connections to hold in backlog'),
    http1_buffer_size: int = typer.Option(
        HTTP1Settings.max_buffer_size, min=8192, help='Set the maximum buffer size for HTTP/1 connections'
    ),
    http1_keep_alive: bool = typer.Option(
        HTTP1Settings.keep_alive,
        '--http1-keep-alive/--no-http1-keep-alive',
        show_default='enabled',
        help='Enables or disables HTTP/1 keep-alive',
    ),
    http1_pipeline_flush: bool = typer.Option(
        HTTP1Settings.pipeline_flush,
        '--http1-pipeline-flush/--no-http1-pipeline-flush',
        show_default='disabled',
        help='Aggregates HTTP/1 flushes to better support pipelined responses (experimental)',
    ),
    http2_adaptive_window: bool = typer.Option(
        HTTP2Settings.adaptive_window,
        '--http2-adaptive-window/--no-http2-adaptive-window',
        show_default='disabled',
        help='Sets whether to use an adaptive flow control for HTTP2',
    ),
    http2_initial_connection_window_size: int = typer.Option(
        HTTP2Settings.initial_connection_window_size, help='Sets the max connection-level flow control for HTTP2'
    ),
    http2_initial_stream_window_size: int = typer.Option(
        HTTP2Settings.initial_stream_window_size,
        help='Sets the `SETTINGS_INITIAL_WINDOW_SIZE` option for HTTP2 stream-level flow control',
    ),
    http2_keep_alive_interval: Optional[int] = typer.Option(
        HTTP2Settings.keep_alive_interval,
        help='Sets an interval for HTTP2 Ping frames should be sent to keep a connection alive',
        show_default='disabled',
    ),
    http2_keep_alive_timeout: int = typer.Option(
        HTTP2Settings.keep_alive_timeout,
        help='Sets a timeout for receiving an acknowledgement of the HTTP2 keep-alive ping',
    ),
    http2_max_concurrent_streams: int = typer.Option(
        HTTP2Settings.max_concurrent_streams,
        help='Sets the SETTINGS_MAX_CONCURRENT_STREAMS option for HTTP2 connections',
    ),
    http2_max_frame_size: int = typer.Option(
        HTTP2Settings.max_frame_size, help='Sets the maximum frame size to use for HTTP2'
    ),
    http2_max_headers_size: int = typer.Option(
        HTTP2Settings.max_headers_size, help='Sets the max size of received header frames'
    ),
    http2_max_send_buffer_size: int = typer.Option(
        HTTP2Settings.max_send_buffer_size, help='Set the maximum write buffer size for each HTTP/2 stream'
    ),
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
    respawn_failed_workers: bool = typer.Option(
        False,
        '--respawn-failed-workers/--no-respawn-failed-workers',
        help='Enable workers respawn on unexpected exit',
        show_default='disabled',
    ),
    reload: bool = typer.Option(
        False, '--reload/--no-reload', help="Enable auto reload on application's files changes", show_default='disabled'
    ),
    _: Optional[bool] = typer.Option(
        None,
        '--version',
        callback=version_callback,
        is_eager=True,
        help='Shows the version and exit',
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
        pthreads=blocking_threads,
        threading_mode=threading_mode,
        loop=loop,
        loop_opt=loop_opt,
        http=http,
        websockets=websockets,
        backlog=backlog,
        http1_settings=HTTP1Settings(
            keep_alive=http1_keep_alive, max_buffer_size=http1_buffer_size, pipeline_flush=http1_pipeline_flush
        ),
        http2_settings=HTTP2Settings(
            adaptive_window=http2_adaptive_window,
            initial_connection_window_size=http2_initial_connection_window_size,
            initial_stream_window_size=http2_initial_stream_window_size,
            keep_alive_interval=http2_keep_alive_interval,
            keep_alive_timeout=http2_keep_alive_timeout,
            max_concurrent_streams=http2_max_concurrent_streams,
            max_frame_size=http2_max_frame_size,
            max_headers_size=http2_max_headers_size,
            max_send_buffer_size=http2_max_send_buffer_size,
        ),
        log_enabled=log_enabled,
        log_level=log_level,
        log_dictconfig=log_dictconfig,
        ssl_cert=ssl_certificate,
        ssl_key=ssl_keyfile,
        url_path_prefix=url_path_prefix,
        respawn_failed_workers=respawn_failed_workers,
        reload=reload,
    ).serve()
