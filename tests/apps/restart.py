import json
import os


def pid(environ, protocol):
    protocol('200 OK', [('content-type', 'text/plain; charset=utf-8')])
    return [
        json.dumps(
            {
                'pid': os.getpid(),
            }
        ).encode('utf8')
    ]


def app(environ, protocol):
    return {'/pid': pid}[environ['PATH_INFO']](environ, protocol)
