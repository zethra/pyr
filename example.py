from pyr import *


@route('/')
def handler():
    return "Hello Rust"


@route('/cake')
def cake():
    return "Cake"


def main():
    router = Router(handler, cake)
    pyr_server = PyrServer("127.0.0.1:3000", router)
    pyr_server.start()
    input("Press key to stop")
    pyr_server.stop()


if __name__ == '__main__':
    main()
