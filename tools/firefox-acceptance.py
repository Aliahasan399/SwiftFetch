"""Exercise the real Firefox add-on, native host and transfer in disposable state.

Uses only the verified runtime/dependencies recorded by prepare-firefox.ps1.
Never replaces an existing native-host registration or the user's browser profile.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import socket
import sqlite3
import subprocess
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from uuid import uuid4
import winreg


ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument("binary", nargs="?", default="src-tauri/target/release/swiftfetch.exe")
parser.add_argument("--runtime", default="artifacts/firefox-runtime-634878161579476eacacbe231b777cc7/runtime.json")
parser.add_argument("--headless", action="store_true", help="Use only where the Firefox headless compositor is available")
args = parser.parse_args()
runtime = json.loads((ROOT / args.runtime).read_text(encoding="utf-8-sig"))
sys.path.insert(0, runtime["python_path"])
from marionette_driver.marionette import Marionette
from marionette_driver.addons import Addons

binary = (ROOT / args.binary).resolve()
work = ROOT / "artifacts" / "firefox-acceptance" / ("run-" + uuid4().hex)
work.mkdir(parents=True)
profile = work / "profile"
profile.mkdir()
downloads = work / "downloads"
downloads.mkdir()
environment = {**os.environ, "SWIFTFETCH_DB_PATH": str(work / "state.sqlite"),
               "SWIFTFETCH_DATA_DIR": str(work), "SWIFTFETCH_DOWNLOADS_DIR": str(downloads),
               "WEBVIEW2_USER_DATA_FOLDER": str(work / "webview")}
key_path = r"Software\Mozilla\NativeMessagingHosts\com.swiftfetch.app"
manifest = work / "com.swiftfetch.app.json"
data = b"SwiftFetch actual Firefox native messaging acceptance\n" * 4096


class Fixture(BaseHTTPRequestHandler):
    def do_GET(self):
        requested = re.fullmatch(r"bytes=(\d+)-(\d*)", self.headers.get("Range", ""))
        start = int(requested[1]) if requested else 0
        end = int(requested[2]) if requested and requested[2] else len(data) - 1
        self.send_response(206 if requested else 200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(end - start + 1))
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("ETag", '"firefox-acceptance-v1"')
        if requested:
            self.send_header("Content-Range", f"bytes {start}-{end}/{len(data)}")
        self.end_headers()
        self.wfile.write(data[start:end + 1])

    def log_message(self, *_):
        pass


def until(description, operation, timeout=35):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        try:
            value = operation()
            if value:
                return value
        except (OSError, sqlite3.Error) as error:
            last = error
        time.sleep(0.1)
    raise RuntimeError(f"Timed out waiting for {description}: {last}")


def enable_bridge():
    with sqlite3.connect(environment["SWIFTFETCH_DB_PATH"], timeout=1) as connection:
        row = connection.execute("SELECT value_json FROM settings WHERE key='app_settings'").fetchone()
        settings = json.loads(row[0]) if row else {}
        settings.update(browser_integration=True, destination_root=str(downloads),
                        clipboard_capture=False, shutdown_after_complete=False)
        connection.execute("INSERT OR REPLACE INTO settings(key,value_json,updated_at)VALUES('app_settings',?,datetime('now'))", (json.dumps(settings),))
        return True


desktop = firefox = client = server = None
registered = False
result = {"passed": False, "isolated_profile": True, "temporary_registration": True}
try:
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path):
            raise RuntimeError("An existing SwiftFetch Firefox native host is registered; the test will not replace it.")
    except FileNotFoundError:
        pass
    packaged = work / "extension"
    subprocess.run(["node", "browser-extension/tools/package.mjs", str(packaged)], cwd=ROOT, check=True, capture_output=True, text=True)
    manifest.write_text(json.dumps({"name": "com.swiftfetch.app", "description": "SwiftFetch isolated Firefox acceptance", "path": str(binary),
                                    "type": "stdio", "allowed_extensions": ["swiftfetch@localhost"]}), encoding="utf-8")
    with winreg.CreateKeyEx(winreg.HKEY_CURRENT_USER, key_path) as key:
        winreg.SetValueEx(key, "", 0, winreg.REG_SZ, str(manifest))
    registered = True
    server = ThreadingHTTPServer(("127.0.0.1", 0), Fixture)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    desktop = subprocess.Popen([str(binary)], cwd=work, env=environment, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    until("desktop queue initialization", enable_bridge)
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    prefs = {"marionette.port": port, "marionette.log.level": "Trace", "remote.log.level": "Trace", "browser.shell.checkDefaultBrowser": False,
             "browser.startup.homepage_override.mstone": "ignore", "browser.startup.page": 0,
             "datareporting.policy.dataSubmissionEnabled": False, "toolkit.telemetry.reportingpolicy.firstRun": False}
    (profile / "user.js").write_text("\n".join(f"user_pref({json.dumps(key)}, {json.dumps(value)});" for key, value in prefs.items()), encoding="utf-8")
    log = (work / "firefox.log").open("w", encoding="utf-8")
    firefox = subprocess.Popen([runtime["firefox"], "-no-remote", "-profile", str(profile), "-marionette", *( ["-headless"] if args.headless else [] ), "about:blank"],
                               cwd=work, env={**environment, "MOZ_REMOTE_ALLOW_SYSTEM_ACCESS": "1"}, stdout=log, stderr=log)

    def available():
        with socket.create_connection(("127.0.0.1", port), timeout=0.2):
            return True

    until("Firefox Marionette listener", available)
    client = Marionette(host="127.0.0.1", port=port, socket_timeout=90)
    capabilities = client.start_session()
    client.timeout.script = 20
    addon_id = Addons(client).install(str(packaged / "firefox"), temp=True)
    assert addon_id == "swiftfetch@localhost", addon_id
    with client.using_context("chrome"):
        origin = client.execute_script("return WebExtensionPolicy.getByID(arguments[0]).mozExtensionHostname;", script_args=[addon_id])
    client.navigate(f"moz-extension://{origin}/popup.html")
    fixture_origin = f"http://127.0.0.1:{server.server_port}"

    def message(payload):
        return client.execute_async_script("const [payload, done]=arguments; browser.runtime.sendMessage(payload).then(done,error=>done({error:String(error)}));",
                                           script_args=[payload], sandbox=None)

    state = message({"type": "swiftfetch_get_state"})
    assert state.get("ok") is True, state
    disabled = message({"type": "swiftfetch_set_enabled", "enabled": False})
    assert disabled.get("ok") is True, disabled
    intent = {"type": "swiftfetch_intent", "source": "popup", "payload": {"url": fixture_origin + "/firefox.bin", "suggestedName": "firefox-acceptance.bin", "pageOrigin": fixture_origin, "contentType": "application/octet-stream"}}
    blocked = message(intent)
    assert blocked.get("code") == "extension_disabled", blocked
    assert message({"type": "swiftfetch_set_enabled", "enabled": True}).get("ok") is True
    reply = message(intent)
    (work / "native-reply.json").write_text(json.dumps(reply, indent=2), encoding="utf-8")
    assert reply.get("ok") is True and reply.get("taskId"), reply
    output = downloads / "firefox-acceptance.bin"
    until("the Firefox-originated output file", lambda: output.exists() and output.read_bytes() == data)
    result.update(passed=True, browser="Firefox", version=capabilities.get("browserVersion"),
                  native_messaging=True, extension_background=True, disable_reenable=True,
                  sha256=hashlib.sha256(output.read_bytes()).hexdigest(), bytes=len(data))
    print(f"PASS actual Firefox add-on load, background enable/disable, native host spawn, queue intake and matching output: {work}", flush=True)
except Exception as error:
    result["error"] = str(error)
    raise
finally:
    (work / "result.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    if client:
        try:
            client.delete_session()
        except Exception:
            pass
    for process in (firefox, desktop):
        if process and process.poll() is None:
            subprocess.run(["taskkill.exe", "/PID", str(process.pid), "/T", "/F"], capture_output=True)
    if server:
        server.shutdown()
        server.server_close()
    if registered:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path) as key:
            current, _ = winreg.QueryValueEx(key, "")
        if current != str(manifest):
            raise RuntimeError("The test host registration changed; it was preserved for review.")
        winreg.DeleteKey(winreg.HKEY_CURRENT_USER, key_path)
