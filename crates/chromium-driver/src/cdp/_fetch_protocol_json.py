#!/usr/bin/env python3
"""Download browser_protocol.json and js_protocol.json, split into one file per domain."""

import json
import urllib.request
import os

BASE = "https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/refs/heads/master/json"
URLS = [
    f"{BASE}/browser_protocol.json",
    f"{BASE}/js_protocol.json",
]
OUT_DIR = os.path.dirname(os.path.abspath(__file__))

seen = set()
total = 0

for url in URLS:
    name = url.rsplit("/", 1)[-1]
    print(f"Downloading {name}...")
    with urllib.request.urlopen(url) as resp:
        protocol = json.loads(resp.read())

    for domain in protocol["domains"]:
        domain_name = domain["domain"]
        if domain_name in seen:
            print(f"  {domain_name}.json (skipped, already from browser_protocol)")
            continue
        seen.add(domain_name)
        path = os.path.join(OUT_DIR, f"{domain_name.lower()}.json")
        with open(path, "w") as f:
            json.dump(domain, f, indent=4)
        print(f"  {domain_name}.json")
        total += 1

print(f"\nDone: {total} domains exported.")
