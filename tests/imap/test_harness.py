"""Regression checks for DNS-independent startup and failure artifacts."""
import asyncio
import contextlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


def module(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + ".py"))
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


runner = module("run")
fixture = module("server")


class HarnessTests(unittest.TestCase):
    def test_failure_still_writes_nonsecret_artifact(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "nested/result.json"
            with patch.object(runner, "run", side_effect=TimeoutError("private-fixture-password")):
                with self.assertRaises(TimeoutError):
                    runner.main(["--output", str(output)])
            result = json.loads(output.read_text())
            self.assertEqual(result["status"], "failed")
            self.assertEqual(result["errorType"], "TimeoutError")
            self.assertNotIn("private-fixture-password", output.read_text())

    def test_success_keeps_scenario_summary(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "result.json"
            def success(args, passed):
                passed.append("scenario passed")
            with patch.object(runner, "run", side_effect=success):
                runner.main(["--output", str(output)])
            self.assertEqual(json.loads(output.read_text())["scenarios"], ["scenario passed"])
            self.assertEqual(json.loads(output.read_text())["status"], "passed")

    def test_greeting_does_not_depend_on_runner_dns(self):
        async def verify():
            with tempfile.TemporaryDirectory() as directory:
                ready = Path(directory) / "ready.json"
                with patch.dict(os.environ), patch("socket.getfqdn", side_effect=AssertionError("DNS must not be called")):
                    os.environ.pop("FQDN", None)
                    task = asyncio.create_task(fixture.serve(str(ready)))
                    writer = None
                    try:
                        for _ in range(100):
                            if task.done():
                                await task
                            if ready.exists():
                                break
                            await asyncio.sleep(0.02)
                        port = json.loads(ready.read_text())["port"]
                        reader, writer = await asyncio.open_connection("127.0.0.1", port)
                        greeting = await asyncio.wait_for(reader.readline(), 2)
                        self.assertTrue(greeting.startswith(b"* OK"))
                        self.assertIn(b"imap-fixture.test", greeting)
                        writer.write(b"t1 NOOP\r\n")
                        await writer.drain()
                        response = await asyncio.wait_for(reader.readline(), 2)
                        self.assertTrue(response.startswith(b"t1 OK"))
                    finally:
                        if writer:
                            writer.close()
                            await writer.wait_closed()
                        task.cancel()
                        with contextlib.suppress(asyncio.CancelledError):
                            await task
        asyncio.run(verify())
