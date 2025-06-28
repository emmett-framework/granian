import ipaddress
from functools import wraps as _wraps


class _Forwarders:
    def __init__(self, trusted_hosts: list[str] | str) -> None:
        self.always_trust: bool = trusted_hosts in ('*', ['*'])
        self.literals: set[str] = set()
        self.hosts: set[ipaddress.IPv4Address | ipaddress.IPv6Address] = set()
        self.networks: set[ipaddress.IPv4Network | ipaddress.IPv6Network] = set()

        if self.always_trust:
            return

        if isinstance(trusted_hosts, str):
            trusted_hosts = _parse_raw_hosts(trusted_hosts)

        for host in trusted_hosts:
            try:
                if '/' in host:
                    self.networks.add(ipaddress.ip_network(host))
                    continue
                self.hosts.add(ipaddress.ip_address(host))
            except ValueError:
                self.literals.add(host)

    def __contains__(self, host: str | None) -> bool:
        if self.always_trust:
            return True

        if not host:
            return False

        try:
            ip = ipaddress.ip_address(host)
            if ip in self.hosts:
                return True
            return any(ip in net for net in self.networks)
        except ValueError:
            return host in self.literals

    def get_client_host(self, x_forwarded_for: str) -> str:
        x_forwarded_for_hosts = _parse_raw_hosts(x_forwarded_for)

        if self.always_trust:
            return x_forwarded_for_hosts[0]

        for host in reversed(x_forwarded_for_hosts):
            if host not in self:
                return host

        return x_forwarded_for_hosts[0]


def _parse_raw_hosts(value: str) -> list[str]:
    return [item.strip() for item in value.split(',')]


def wrap_asgi_with_proxy_headers(app, trusted_hosts: list[str] | str = '127.0.0.1'):
    forwarders = _Forwarders(trusted_hosts)

    @_wraps(app)
    def wrapped(scope, receive, send):
        if scope['type'] == 'lifespan':
            return app(scope, receive, send)

        client_addr = scope.get('client')
        client_host = client_addr[0] if client_addr else None

        if client_host in forwarders:
            headers = dict(scope['headers'])

            if x_forwarded_proto := headers.get(b'x-forwarded-proto', b'').decode('latin1').strip():
                if x_forwarded_proto in {'http', 'https', 'ws', 'wss'}:
                    if scope['type'] == 'websocket':
                        scope['scheme'] = x_forwarded_proto.replace('http', 'ws')
                    else:
                        scope['scheme'] = x_forwarded_proto

            if x_forwarded_for := headers.get(b'x-forwarded-for', b'').decode('latin1'):
                if host := forwarders.get_client_host(x_forwarded_for):
                    scope['client'] = (host, 0)

        return app(scope, receive, send)

    return wrapped


def wrap_wsgi_with_proxy_headers(app, trusted_hosts: list[str] | str = '127.0.0.1'):
    forwarders = _Forwarders(trusted_hosts)

    @_wraps(app)
    def wrapped(scope, resp):
        client_host = scope.get('REMOTE_ADDR')

        if client_host in forwarders:
            if x_forwarded_proto := scope.get('HTTP_X_FORWARDED_PROTO'):
                if x_forwarded_proto in {'http', 'https'}:
                    scope['wsgi.url_scheme'] = x_forwarded_proto

            if x_forwarded_for := scope.get('HTTP_X_FORWARDED_FOR'):
                if host := forwarders.get_client_host(x_forwarded_for):
                    scope['REMOTE_ADDR'] = host

        return app(scope, resp)

    return wrapped
