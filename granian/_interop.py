import abc

from ._granian import App as _App


class AppBuilder:
    def build(self, access_log: bool = False) -> _App:
        if access_log:
            return _App(self.on_request_wlog, self.on_websocket_wlog)
        return _App(self.on_request, self.on_websocket)


class App(AppBuilder, abc.ABC):
    @abc.abstractmethod
    def on_request(self, proto, scope) -> None: ...

    @abc.abstractmethod
    def on_websocket(self, proto, scope) -> None: ...

    @abc.abstractmethod
    def on_request_wlog(self, proto, scope) -> None: ...

    @abc.abstractmethod
    def on_websocket_wlog(self, proto, scope) -> None: ...
