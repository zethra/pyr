import libpyr


class PyrServer:
    def __init__(self):
        self._server_handel = None

    def start(self, handler_fn):
        self._server_handel = libpyr.start_server(handler_fn)

    def stop(self):
        libpyr.stop_server(self._server_handel)


def handler():
    return "Hello Rust"


def main():
    pyr_server = PyrServer()
    pyr_server.start(handler)
    input("Press key to stop")
    pyr_server.stop()


if __name__ == '__main__':
    main()
