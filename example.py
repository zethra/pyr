from pyr import *
import time
import email.utils


@route('/')
def handler():
    print("127.0.0.1 - - [{}] \"GET / HTTP/1.1\" 200 -".format(email.utils.formatdate(time.time(), usegmt=True)))
    return "Hello Rust"


@route('/cake')
def cake():
    return "Cake"


def main():
    router = Router(handler, cake)
    pyr_server = PyrServer("127.0.0.1:3000", router)
    pyr_server.start()
    input("Press key to stop\n")
    pyr_server.stop()


if __name__ == '__main__':
    main()
