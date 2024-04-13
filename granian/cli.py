import json
import pathlib
from typing import Any, Callable, Literal, Optional, Type, TypeVar, Union

import click

from .constants import HTTPModes, Interfaces, Loops, ThreadModes
from .http import HTTP1Settings, HTTP2Settings
from .log import LogLevels
from .server import Granian


_AnyCallable = Callable[..., Any]
FC = TypeVar('FC', bound=Union[_AnyCallable, click.Command])


def show_env_var(*param_decls: str, cls: Optional[Type[click.Option]] = None, **attrs: Any) -> Callable[[FC], FC]:
    """This is the option generator from Click.

    It has a very simple update so that the environment variable will also get added to the help text.
    """
    if cls is None:
        cls = click.Option

    def decorator(f: FC) -> FC:
        # start change
        env_name = param_decls[-1].lstrip('-').split('/')[0].replace('-', '_').upper()
        attrs['help'] += f' [env var: GRANIAN_{env_name}]'
        # end change
        click.decorators._param_memo(f, cls(param_decls, **attrs))
        return f

    return decorator


# monkey patch so that the behaviour is more similar to the docs.
click.option = show_env_var


@click.command(context_settings={'show_default': True})
@click.argument('app', required=True, type=str)
@click.option(
    '--host',
    type=str,
    default='127.0.0.1',
    help='Host address to bind to',
)
@click.option(
    '--port',
    type=int,
    default=8000,
    help='Port to bind to.',
)
@click.option(
    '--interface',
    type=click.Choice(['asgi', 'asginl', 'rsgi', 'wsgi']),
    default='rsgi',
    help='Application interface type',
)
@click.option(
    '--http',
    type=click.Choice(['auto', '1', '2']),
    default='auto',
    help='HTTP version',
)
@click.option(
    '--ws/--no-ws',
    'websockets',
    type=bool,
    default=False,
    help='Enable websockets handling.',
    show_default='enabled',
)
@click.option(
    '--workers',
    type=click.IntRange(1),
    default=1,
    help='Number of worker processes',
)
@click.option(
    '--threads',
    type=click.IntRange(1),
    default=1,
    help='Number of threads',
)
@click.option(
    '--blocking-threads',
    type=click.IntRange(1),
    default=1,
    help='Number of blocking threads',
)
@click.option(
    '--threading-mode',
    type=click.Choice(['runtime', 'workers']),
    default='workers',
    help='Threading mode to use',
)
@click.option(
    '--loop',
    type=click.Choice(['auto', 'asyncio', 'uvloop']),
    default='auto',
    help='Event loop implementation',
)
@click.option(
    '--opt/--no-opt',
    'loop_opt',
    default=False,
    show_default='disabled',
    help='Enable loop optimizations',
)
@click.option(
    '--backlog',
    type=click.IntRange(128),
    default=1024,
    help='Maximum number of connections to hold in backlog',
)
@click.option(
    '--http1-buffer-size',
    type=click.IntRange(8192),
    default=HTTP1Settings.max_buffer_size,
    help='Set the maximum buffer size for HTTP/1 connections',
)
@click.option(
    '--http1-keep-alive/--no-http1-keep-alive',
    show_default='enabled',
    default=HTTP1Settings.keep_alive,
    help='Enables or disables HTTP/1 keep-alive',
)
@click.option(
    '--http1-pipeline-flush/--no-http1-pipeline-flush',
    show_default='disabled',
    default=HTTP1Settings.pipeline_flush,
    help='Aggregates HTTP/1 flushes to better support pipelined responses (experimental)',
)
@click.option(
    '--http2-adaptive-window/--no-http2-adaptive-window',
    show_default='disabled',
    default=HTTP2Settings.adaptive_window,
    help='Sets whether to use an adaptive flow control for HTTP2',
)
@click.option(
    '--http2-initial-connection-window-size',
    type=int,
    default=HTTP2Settings.initial_connection_window_size,
    help='Sets the max connection-level flow control for HTTP2',
)
@click.option(
    '--http2-initial-stream-window-size',
    type=int,
    default=HTTP2Settings.initial_stream_window_size,
    help='Sets the `SETTINGS_INITIAL_WINDOW_SIZE` option for HTTP2 stream-level flow control',
)
@click.option(
    '--http2-keep-alive-interval',
    type=int,
    default=HTTP2Settings.keep_alive_interval,
    show_default='disabled',
    help='Sets an interval for HTTP2 Ping frames should be sent to keep a connection alive',
)
@click.option(
    '--http2-keep-alive-timeout',
    type=int,
    default=HTTP2Settings.keep_alive_timeout,
    help='Sets a timeout for receiving an acknowledgement of the HTTP2 keep-alive ping',
)
@click.option(
    '--http2-max-concurrent-streams',
    type=int,
    default=HTTP2Settings.max_concurrent_streams,
    help='Sets the SETTINGS_MAX_CONCURRENT_STREAMS option for HTTP2 connections',
)
@click.option(
    '--http2-max-frame-size',
    type=int,
    default=HTTP2Settings.max_frame_size,
    help='Sets the maximum frame size to use for HTTP2',
)
@click.option(
    '--http2-max-headers-size',
    type=int,
    default=HTTP2Settings.max_headers_size,
    help='Sets the max size of received header frames',
)
@click.option(
    '--http2-max-send-buffer-size',
    type=int,
    default=HTTP2Settings.max_send_buffer_size,
    help='Set the maximum write buffer size for each HTTP/2 stream',
)
@click.option(
    '--log/--no-log',
    'log_enabled',
    show_default='enabled',
    default=True,
    help='Enable logging',
)
@click.option(
    '--log-level',
    type=click.Choice(['critical', 'error', 'warning', 'warn', 'info', 'debug'], case_sensitive=False),
    default='info',
    help='Log level.',
)
@click.option(
    '--log-config',
    type=click.Path(exists=True, file_okay=True, dir_okay=True, readable=True, path_type=pathlib.Path),
    help='Logging configuration file (json)',
    default=None,
)
@click.option(
    '--ssl-keyfile',
    type=click.Path(exists=True, file_okay=True, dir_okay=False, readable=True, path_type=pathlib.Path),
    default=None,
    help='SSL key file',
)
@click.option(
    '--ssl-certificate',
    type=click.Path(exists=True, file_okay=True, dir_okay=False, readable=True, path_type=pathlib.Path),
    default=None,
    help='SSL certificate file',
)
@click.option('--url-path-prefix', type=str, default=None, help='URL path prefix the app is mounted on')
@click.option(
    '--respawn-failed-workers/--no-respawn-failed-workers',
    default=False,
    show_default='disabled',
    help='Enable workers respawn on unexpected exit',
)
@click.option(
    '--reload/--no-reload',
    default=False,
    help="Enable auto reload on application's files changes (requires granian[reload] extra)",
    show_default='disabled',
)
@click.option(
    '--process-name',
    type=str,
    default=None,
    help='Set a custom name for processes (requires granian[pname] extra)',
)
@click.version_option(message='%(prog)s %(version)s')
def cli(
    app: str,
    host: str,
    port: int,
    interface: Literal['asgi', 'asginl', 'rsgi', 'wsgi'],
    http: Literal['auto', '1', '2'],
    websockets: bool,
    workers: int,
    threads: int,
    blocking_threads: int,
    threading_mode: Literal['runtime', 'workers'],
    loop: Literal['auto', 'asyncio', 'uvloop'],
    loop_opt: bool,
    backlog: int,
    http1_buffer_size: int,
    http1_keep_alive: bool,
    http1_pipeline_flush: bool,
    http2_adaptive_window: bool,
    http2_initial_connection_window_size: int,
    http2_initial_stream_window_size: int,
    http2_keep_alive_interval: Optional[int],
    http2_keep_alive_timeout: int,
    http2_max_concurrent_streams: int,
    http2_max_frame_size: int,
    http2_max_headers_size: int,
    http2_max_send_buffer_size: int,
    log_enabled: bool,
    log_level: Literal['critical', 'error', 'warning', 'warn', 'info', 'debug'],
    log_config: Optional[pathlib.Path],
    ssl_keyfile: Optional[pathlib.Path],
    ssl_certificate: Optional[pathlib.Path],
    url_path_prefix: Optional[str],
    respawn_failed_workers: bool,
    reload: bool,
    process_name: Optional[str],
) -> None:
    """
    APP:  Application target to serve.  [required]
    """
    log_dictconfig = None
    if log_config:
        with log_config.open() as log_config_file:
            try:
                log_dictconfig = json.loads(log_config_file.read())
            except Exception:
                print('Unable to parse provided logging config.')
                raise click.exceptions.Exit(1)

    Granian(
        app,
        address=host,
        port=port,
        interface=Interfaces(interface),
        workers=workers,
        threads=threads,
        pthreads=blocking_threads,
        threading_mode=ThreadModes(threading_mode),
        loop=Loops(loop),
        loop_opt=loop_opt,
        http=HTTPModes(http),
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
        log_level=LogLevels(log_level),
        log_dictconfig=log_dictconfig,
        ssl_cert=ssl_certificate,
        ssl_key=ssl_keyfile,
        url_path_prefix=url_path_prefix,
        respawn_failed_workers=respawn_failed_workers,
        reload=reload,
        process_name=process_name,
    ).serve()


# make sure the correct env var prefix is being used.
def entrypoint() -> None:
    cli(auto_envvar_prefix='GRANIAN')
