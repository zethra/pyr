import libpyr


class PyrServer:
    def __init__(self):
        self._server_handel = None

    def start(self, routes):
        self._server_handel = libpyr.start_server(routes)

    def stop(self):
        libpyr.stop_server(self._server_handel)


def handler():
    return "Hello Rust"


def cake():
    return "Cake"


def main():
    routes = {
        '/': handler,
        '/cake': cake
    }
    pyr_server = PyrServer()
    pyr_server.start(routes)
    input("Press key to stop")
    pyr_server.stop()


if __name__ == '__main__':
    main()
