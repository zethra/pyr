import libpyr
from enum import Enum


class RequestType(Enum):
    ANY = 'ANY'
    GET = 'GET'
    POST = 'POST'


class route:
    def __init__(self, path, request_type=RequestType.ANY):
        self.path = path
        self.request_type = request_type

    def __call__(self, handler_fn):
        return libpyr.Route(self.path, handler_fn, self.request_type.value)


class Router:
    def __init__(self, *args):
        self.routes = []
        for arg in args:
            self.routes.append(arg)


class PyrServer:
    def __init__(self, router):
        self.router = router
        self._server_handel = None

    def start(self):
        self._server_handel = libpyr.start_server(self.router.routes)

    def stop(self):
        libpyr.stop_server(self._server_handel)


def main():
    req = libpyr.PyRequest(123)


if __name__ == '__main__':
    main()
