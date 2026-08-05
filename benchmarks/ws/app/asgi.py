import asyncio

clients = set()


def broadcast(message):
    tasks = []
    for ws in list(clients):
        tasks.append(
            asyncio.ensure_future(
                ws({'type': 'websocket.send', 'bytes': message, 'text': None})
            )
        )
    return asyncio.gather(*tasks, return_exceptions=True)


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
