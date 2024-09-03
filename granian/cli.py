import json
import pathlib
from enum import Enum
from typing import Any, Callable, List, Optional, Type, TypeVar, Union

import click

from .constants import HTTPModes, Interfaces, Loops, ThreadModes
from .errors import FatalError
from .http import HTTP1Settings, HTTP2Settings
from .log import LogLevels
from .server import Granian


_AnyCallable = Callable[..., Any]
FC = TypeVar('FC', bound=Union[_AnyCallable, click.Command])


class EnumType(click.Choice):
    def __init__(self, enum: Enum, case_sensitive=False) -> None:
        self.__enum = enum
        super().__init__(choices=[item.value for item in enum], case_sensitive=case_sensitive)

    def convert(self, value: Any, param: Optional[click.Parameter], ctx: Optional[click.Context]) -> Enum:
        if value is None or isinstance(value, Enum):
            return value

        converted_str = super().convert(value, param, ctx)
        return self.__enum(converted_str)


def _pretty_print_default(value: Optional[bool]) -> Optional[str]:
    if isinstance(value, bool):
        return 'enabled' if value else 'disabled'
    if isinstance(value, Enum):
        return value.value
    return value


def option(*param_decls: str, cls: Optional[Type[click.Option]] = None, **attrs: Any) -> Callable[[FC], FC]:
    attrs['show_envvar'] = True
    if 'default' in attrs:
        attrs['show_default'] = _pretty_print_default(attrs['default'])
    return click.option(*param_decls, cls=cls, **attrs)


@click.command(
    context_settings={'show_default': True},
    help='APP  Application target to serve.  [required]',
)
@click.argument('app', required=True)
@option(
    '--host',
    default='127.0.0.1',
    help='Host address to bind to',
)
@option('--port', type=int, default=8000, help='Port to bind to.')
@option(
    '--interface',
    type=EnumType(Interfaces),
    default=Interfaces.RSGI,
    help='Application interface type',
)
@option('--http', type=EnumType(HTTPModes), default=HTTPModes.auto, help='HTTP version')
@option('--ws/--no-ws', 'websockets', default=True, help='Enable websockets handling')
@option('--workers', type=click.IntRange(1), default=1, help='Number of worker processes')
@option('--threads', type=click.IntRange(1), default=1, help='Number of threads (per worker)')
@option(
    '--blocking-threads',
    type=click.IntRange(1),
    help='Number of blocking threads (per worker)',
)
@option(
    '--threading-mode',
    type=EnumType(ThreadModes),
    default=ThreadModes.workers,
    help='Threading mode to use',
)
@option('--loop', type=EnumType(Loops), default=Loops.auto, help='Event loop implementation')
@option('--opt/--no-opt', 'loop_opt', default=False, help='Enable loop optimizations')
@option(
    '--backlog',
    type=click.IntRange(128),
    default=1024,
    help='Maximum number of connections to hold in backlog (globally)',
)
@option(
    '--backpressure',
    type=click.IntRange(1),
    show_default='backlog/workers',
    help='Maximum number of requests to process concurrently (per worker)',
)
@option(
    '--http1-buffer-size',
    type=click.IntRange(8192),
    default=HTTP1Settings.max_buffer_size,
    help='Set the maximum buffer size for HTTP/1 connections',
)
@option(
    '--http1-keep-alive/--no-http1-keep-alive',
    default=HTTP1Settings.keep_alive,
    help='Enables or disables HTTP/1 keep-alive',
)
@option(
    '--http1-pipeline-flush/--no-http1-pipeline-flush',
    default=HTTP1Settings.pipeline_flush,
    help='Aggregates HTTP/1 flushes to better support pipelined responses (experimental)',
)
@option(
    '--http2-adaptive-window/--no-http2-adaptive-window',
    default=HTTP2Settings.adaptive_window,
    help='Sets whether to use an adaptive flow control for HTTP2',
)
@option(
    '--http2-initial-connection-window-size',
    type=int,
    default=HTTP2Settings.initial_connection_window_size,
    help='Sets the max connection-level flow control for HTTP2',
)
@option(
    '--http2-initial-stream-window-size',
    type=int,
    default=HTTP2Settings.initial_stream_window_size,
    help='Sets the `SETTINGS_INITIAL_WINDOW_SIZE` option for HTTP2 stream-level flow control',
)
@option(
    '--http2-keep-alive-interval',
    type=int,
    default=HTTP2Settings.keep_alive_interval,
    help='Sets an interval for HTTP2 Ping frames should be sent to keep a connection alive',
)
@option(
    '--http2-keep-alive-timeout',
    type=int,
    default=HTTP2Settings.keep_alive_timeout,
    help='Sets a timeout for receiving an acknowledgement of the HTTP2 keep-alive ping',
)
@option(
    '--http2-max-concurrent-streams',
    type=int,
    default=HTTP2Settings.max_concurrent_streams,
    help='Sets the SETTINGS_MAX_CONCURRENT_STREAMS option for HTTP2 connections',
)
@option(
    '--http2-max-frame-size',
    type=int,
    default=HTTP2Settings.max_frame_size,
    help='Sets the maximum frame size to use for HTTP2',
)
@option(
    '--http2-max-headers-size',
    type=int,
    default=HTTP2Settings.max_headers_size,
    help='Sets the max size of received header frames',
)
@option(
    '--http2-max-send-buffer-size',
    type=int,
    default=HTTP2Settings.max_send_buffer_size,
    help='Set the maximum write buffer size for each HTTP/2 stream',
)
@option('--log/--no-log', 'log_enabled', default=True, help='Enable logging')
@option('--log-level', type=EnumType(LogLevels), default=LogLevels.info, help='Log level')
@option(
    '--log-config',
    type=click.Path(exists=True, file_okay=True, dir_okay=False, readable=True, path_type=pathlib.Path),
    help='Logging configuration file (json)',
)
@option('--access-log/--no-access-log', 'log_access_enabled', default=False, help='Enable access log')
@option('--access-log-fmt', 'log_access_fmt', help='Access log format')
@option(
    '--ssl-certificate',
    type=click.Path(exists=True, file_okay=True, dir_okay=False, readable=True, path_type=pathlib.Path),
    help='SSL certificate file',
)
@option(
    '--ssl-keyfile',
    type=click.Path(exists=True, file_okay=True, dir_okay=False, readable=True, path_type=pathlib.Path),
    help='SSL key file',
)
@option('--ssl-keyfile-password', help='SSL key password')
@option('--url-path-prefix', help='URL path prefix the app is mounted on')
@option(
    '--respawn-failed-workers/--no-respawn-failed-workers',
    default=False,
    help='Enable workers respawn on unexpected exit',
)
@option(
    '--respawn-interval',
    default=3.5,
    help='The number of seconds to sleep between workers respawn',
)
@option(
    '--workers-lifetime',
    type=click.IntRange(60),
    help='The maximum amount of time in seconds a worker will be kept alive before respawn',
)
@option(
    '--factory/--no-factory',
    default=False,
    help='Treat target as a factory function, that should be invoked to build the actual target',
)
@option(
    '--reload/--no-reload',
    default=False,
    help="Enable auto reload on application's files changes (requires granian[reload] extra)",
)
@option(
    '--reload-paths',
    type=click.Path(exists=True, file_okay=True, dir_okay=True, readable=True, path_type=pathlib.Path),
    help='Paths to watch for changes',
    show_default='Working directory',
    multiple=True,
)
@option(
    '--reload-ignore-dirs',
    help=(
        'Names of directories to ignore changes for. '
        "Extends the default list of directories to ignore in watchfiles' default filter"
    ),
    multiple=True,
)
@option(
    '--reload-ignore-patterns',
    help=(
        'File/directory name patterns (regex) to ignore changes for. '
        "Extends the default list of patterns to ignore in watchfiles' default filter"
    ),
    multiple=True,
)
@option(
    '--reload-ignore-paths',
    type=click.Path(exists=False, path_type=pathlib.Path),
    help='Absolute paths to ignore changes for',
    multiple=True,
)
@option(
    '--process-name',
    help='Set a custom name for processes (requires granian[pname] extra)',
)
@option(
    '--pid-file',
    type=click.Path(exists=False, file_okay=True, dir_okay=False, writable=True, path_type=pathlib.Path),
    help='A path to write the PID file to',
)
@click.version_option(message='%(prog)s %(version)s')
def cli(
    app: str,
    host: str,
    port: int,
    interface: Interfaces,
    http: HTTPModes,
    websockets: bool,
    workers: int,
    threads: int,
    blocking_threads: Optional[int],
    threading_mode: ThreadModes,
    loop: Loops,
    loop_opt: bool,
    backlog: int,
    backpressure: Optional[int],
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
    log_access_enabled: bool,
    log_access_fmt: Optional[str],
    log_level: LogLevels,
    log_config: Optional[pathlib.Path],
    ssl_certificate: Optional[pathlib.Path],
    ssl_keyfile: Optional[pathlib.Path],
    ssl_keyfile_password: Optional[str],
    url_path_prefix: Optional[str],
    respawn_failed_workers: bool,
    respawn_interval: float,
    workers_lifetime: Optional[int],
    factory: bool,
    reload: bool,
    reload_paths: Optional[List[pathlib.Path]],
    reload_ignore_dirs: Optional[List[str]],
    reload_ignore_patterns: Optional[List[str]],
    reload_ignore_paths: Optional[List[pathlib.Path]],
    process_name: Optional[str],
    pid_file: Optional[pathlib.Path],
) -> None:
    log_dictconfig = None
    if log_config:
        with log_config.open() as log_config_file:
            try:
                log_dictconfig = json.loads(log_config_file.read())
            except Exception:
                print('Unable to parse provided logging config.')
                raise click.exceptions.Exit(1)

    server = Granian(
        app,
        address=host,
        port=port,
        interface=interface,
        workers=workers,
        threads=threads,
        blocking_threads=blocking_threads,
        threading_mode=threading_mode,
        loop=loop,
        loop_opt=loop_opt,
        http=http,
        websockets=websockets,
        backlog=backlog,
        backpressure=backpressure,
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
        log_access=log_access_enabled,
        log_access_format=log_access_fmt,
        ssl_cert=ssl_certificate,
        ssl_key=ssl_keyfile,
        ssl_key_password=ssl_keyfile_password,
        url_path_prefix=url_path_prefix,
        respawn_failed_workers=respawn_failed_workers,
        respawn_interval=respawn_interval,
        workers_lifetime=workers_lifetime,
        factory=factory,
        reload=reload,
        reload_paths=reload_paths,
        reload_ignore_paths=reload_ignore_paths,
        reload_ignore_dirs=reload_ignore_dirs,
        reload_ignore_patterns=reload_ignore_patterns,
        process_name=process_name,
        pid_file=pid_file,
    )

    try:
        server.serve()
    except FatalError:
        raise click.exceptions.Exit(1)


def entrypoint():
    cli(auto_envvar_prefix='GRANIAN')
