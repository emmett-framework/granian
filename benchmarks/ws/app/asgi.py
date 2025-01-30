clients = set()


async def broadcast(message):
    for ws in list(clients):
        await ws({'type': 'websocket.send', 'bytes': message, 'text': None})


async def app(scope, receive, send):
    try:
        await send({'type': 'websocket.accept'})
        clients.add(send)

        while True:
            msg = await receive()
            if msg['type'] == 'websocket.connect':
                continue
            if msg['type'] == 'websocket.disconnect':
                break
            await broadcast(msg['bytes'])

    finally:
        clients.remove(send)


async_app = app
