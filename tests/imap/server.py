"""Disposable loopback-only IMAP server backed by pymap's in-memory store."""
import argparse
import asyncio
import json
import os
from pathlib import Path

from proxyprotocol.reader import ProxyProtocolReader
from pymap.backend.dict import DictBackend, Identity
from pymap.imap import IMAPServer, IMAPService
from pymap.user import Passwords, UserMetadata


async def serve(ready, cert=None, key=None):
    # pymap otherwise calls socket.getfqdn() synchronously for its greeting.
    # Hosted runner reverse DNS can stall the event loop before the first reply.
    os.environ["FQDN"] = "imap-fixture.test"
    parser = argparse.ArgumentParser()
    parser.set_defaults(debug=False, passlib_cfg=None)
    IMAPService.add_arguments(parser)
    DictBackend.add_subparser("dict", parser.add_subparsers(dest="backend"))
    options = ["--host", "127.0.0.1", "--port", "0"]
    options += ["--cert", cert, "--key", key] if cert else ["--no-tls"]
    args = parser.parse_args(options + ["dict"])
    backend, config = await DictBackend.init(args)
    config.apply_context()
    for username in ("alice", "bob", "carol"):
        identity = Identity(username, backend.login, None, frozenset())
        password = await Passwords(config).hash_password("fixture-" + username)
        await identity.set(UserMetadata(config, username, password=password))
    callback = ProxyProtocolReader(config.proxy_protocol).get_callback(IMAPServer(backend.login, config))
    async with await asyncio.start_server(callback, "127.0.0.1", 0) as server:
        Path(ready).write_text(json.dumps({"port": server.sockets[0].getsockname()[1]}))
        await server.serve_forever()


if __name__ == "__main__":
    cli = argparse.ArgumentParser()
    cli.add_argument("--ready", required=True)
    cli.add_argument("--cert")
    cli.add_argument("--key")
    options = cli.parse_args()
    asyncio.run(serve(options.ready, options.cert, options.key))
