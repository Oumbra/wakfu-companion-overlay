"""Planche de contrôle HTML : le seul rendu visuel exploitable par l'utilisateur.

Le poste de travail est en mode terminal — une image lue en ligne n'y est pas visible
(cf. CLAUDE.md). Toute vérification visuelle passe donc par un fragment HTML publié en
Artifact, images embarquées en base64 (aucune ressource externe n'est chargeable).
"""
from __future__ import annotations

import base64
import html
import io
import json

import numpy as np
from PIL import Image

CSS = """
:root{--bg:#f6f5f3;--fg:#1c1a17;--muted:#6b665e;--line:#dcd7cf;--card:#fffdfa;--accent:#8a6d3b}
@media (prefers-color-scheme: dark){:root:not([data-theme="light"]){--bg:#161513;--fg:#ece8e1;--muted:#9b948a;--line:#332f2a;--card:#1e1c19;--accent:#d8b878}}
:root[data-theme="dark"]{--bg:#161513;--fg:#ece8e1;--muted:#9b948a;--line:#332f2a;--card:#1e1c19;--accent:#d8b878}
body{background:var(--bg);color:var(--fg);font:14px/1.5 ui-sans-serif,system-ui,"Segoe UI",sans-serif;margin:0;padding:32px}
h1{font-size:20px;margin:0 0 4px}
.sub{color:var(--muted);margin:0 0 28px}
.item{background:var(--card);border:1px solid var(--line);border-radius:10px;padding:16px;margin-bottom:18px}
.item h2{font-size:15px;margin:0 0 12px;font-family:ui-monospace,SFMono-Regular,Consolas,monospace}
.row{display:flex;gap:18px;flex-wrap:wrap;align-items:flex-start}
.cell{display:flex;flex-direction:column;gap:6px}
.lab{font-size:11px;text-transform:uppercase;letter-spacing:.06em;color:var(--muted)}
.frame{border:1px solid var(--line);border-radius:6px;padding:10px;display:inline-block;overflow-x:auto;max-width:100%}
.checker{background-image:linear-gradient(45deg,#bbb 25%,transparent 25%),linear-gradient(-45deg,#bbb 25%,transparent 25%),linear-gradient(45deg,transparent 75%,#bbb 75%),linear-gradient(-45deg,transparent 75%,#bbb 75%);background-size:12px 12px;background-position:0 0,0 6px,6px -6px,-6px 0;background-color:#eee}
.dark{background:#2b2f36}.light{background:#efeae1}
img{display:block;image-rendering:pixelated;max-width:none}
table{border-collapse:collapse;font-family:ui-monospace,Consolas,monospace;font-size:12px}
td,th{border:1px solid var(--line);padding:3px 8px;text-align:left}
pre{background:var(--bg);border:1px solid var(--line);border-radius:6px;padding:10px;overflow-x:auto;font-size:12px}
.sw{width:26px;height:18px;border-radius:3px;border:1px solid var(--line);display:inline-block;vertical-align:-4px}
"""


def _b64(arr: np.ndarray) -> str:
    buf = io.BytesIO()
    Image.fromarray(arr.astype(np.uint8), "RGBA").save(buf, "PNG")
    return base64.b64encode(buf.getvalue()).decode()


def img_tag(arr: np.ndarray, scale: int = 1) -> str:
    h, w = arr.shape[:2]
    return (f'<img src="data:image/png;base64,{_b64(arr)}" '
            f'width="{w * scale}" height="{h * scale}" alt="">')


def cell(label: str, arr: np.ndarray, scale: int = 1, bg: str = "checker") -> str:
    return (f'<div class="cell"><span class="lab">{html.escape(label)}</span>'
            f'<div class="frame {bg}">{img_tag(arr, scale)}</div></div>')


def item(name: str, cells: list[str], meta: dict | None = None) -> str:
    body = f'<div class="row">{"".join(cells)}</div>'
    if meta:
        body += f"<pre>{html.escape(json.dumps(meta, ensure_ascii=False, indent=2))}</pre>"
    return f'<div class="item"><h2>{html.escape(name)}</h2>{body}</div>'


def page(title: str, subtitle: str, items: list[str]) -> str:
    return (f"<title>{html.escape(title)}</title>\n<style>{CSS}</style>\n"
            f"<h1>{html.escape(title)}</h1>\n<p class=\"sub\">{html.escape(subtitle)}</p>\n"
            + "\n".join(items))
