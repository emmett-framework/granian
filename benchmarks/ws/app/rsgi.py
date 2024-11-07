from granian.rsgi import WebsocketMessageType


clients = set()


async def broadcast(message):
    for ws in list(clients):
        await ws.send_bytes(message)


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
