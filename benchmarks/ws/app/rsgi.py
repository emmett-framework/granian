import asyncio

from granian.rsgi import WebsocketMessageType


clients = set()


def broadcast(message):
    tasks = []
    for ws in list(clients):
        tasks.append(asyncio.ensure_future(ws.send_bytes(message)))
    return asyncio.gather(*tasks, return_exceptions=True)


async def app(scope, protocol):
    trx = await protocol.accept()
    clients.add(trx)

    try:
        while True:
            message = await trx.receive()
            if message.kind == WebsocketMessageType.close:
                break
            await broadcast(message.data)

    finally:
        clients.remove(trx)
        protocol.close()
