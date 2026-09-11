"""Rendu d'une specification d'interface en page de maquette cotee.

Entree : un JSON redige par le modele a partir des mesures de `uispec` (voir
`references/spec-format.md`). Sortie : un fragment HTML prêt pour l'outil Artifact —
plan cote en SVG, inventaire des blocs, jetons de design.
"""
from __future__ import annotations

import html
import json

CSS = """
:root{
  --ink:#22201d; --ink-soft:#6f695f; --ground:#f7f5f1; --panel:#fffefb;
  --rule:#ded7cc; --grid:#e9e3d9; --accent:#9a6b2f; --measure:#b0552f;
  --chip:#efe9dd;
}
@media (prefers-color-scheme: dark){:root:not([data-theme="light"]){
  --ink:#efeae1; --ink-soft:#a49c90; --ground:#15140f; --panel:#1e1c17;
  --rule:#39342c; --grid:#2a261f; --accent:#d8ab63; --measure:#e08a5c;
  --chip:#272319;
}}
:root[data-theme="dark"]{
  --ink:#efeae1; --ink-soft:#a49c90; --ground:#15140f; --panel:#1e1c17;
  --rule:#39342c; --grid:#2a261f; --accent:#d8ab63; --measure:#e08a5c;
  --chip:#272319;
}
body{background:var(--ground);color:var(--ink);margin:0;padding:36px 32px 64px;
  font:14px/1.55 ui-sans-serif,system-ui,"Segoe UI",sans-serif}
.wrap{max-width:1100px;margin:0 auto;display:flex;flex-direction:column;gap:28px}
header h1{font-size:23px;margin:0 0 6px;letter-spacing:-.01em;text-wrap:balance}
header p{margin:0;color:var(--ink-soft);max-width:62ch}
header .src{font-family:ui-monospace,Consolas,monospace;font-size:12px;color:var(--ink-soft);margin-top:8px}
section{display:flex;flex-direction:column;gap:12px}
h2{font-size:12px;text-transform:uppercase;letter-spacing:.09em;color:var(--ink-soft);
  margin:0;font-weight:600}
.plan{background:var(--panel);border:1px solid var(--rule);border-radius:8px;
  padding:20px;overflow-x:auto}
svg{display:block;max-width:none}
table{border-collapse:collapse;font-size:12.5px;width:100%}
th{text-align:left;font-weight:600;color:var(--ink-soft);font-size:11px;
  text-transform:uppercase;letter-spacing:.06em;padding:6px 10px;border-bottom:1px solid var(--rule)}
td{padding:7px 10px;border-bottom:1px solid var(--grid);vertical-align:top;
  font-variant-numeric:tabular-nums}
td.mono,th.mono{font-family:ui-monospace,Consolas,monospace}
.scroll{overflow-x:auto}
.tokens{display:flex;flex-wrap:wrap;gap:10px}
.tok{display:flex;align-items:center;gap:8px;background:var(--chip);border-radius:999px;
  padding:5px 12px 5px 6px;font-family:ui-monospace,Consolas,monospace;font-size:12px}
.dot{width:18px;height:18px;border-radius:50%;border:1px solid var(--rule)}
.scale{display:flex;align-items:flex-end;gap:10px;flex-wrap:wrap}
.scale div{display:flex;flex-direction:column;align-items:center;gap:5px;
  font-family:ui-monospace,Consolas,monospace;font-size:11px;color:var(--ink-soft)}
.scale i{display:block;background:var(--accent);height:14px;border-radius:2px}
.note{color:var(--ink-soft)}
"""

ROLE_STYLE = {
    "surface": ("var(--rule)", "none", "4 0"),
    "group": ("var(--accent)", "none", "5 3"),
    "control": ("var(--ink-soft)", "none", "4 0"),
    "text": ("var(--ink-soft)", "none", "2 3"),
    "icon": ("var(--accent)", "none", "2 2"),
}


def _esc(s):
    return html.escape(str(s))


def _flat(nodes, parent=None, depth=0, out=None, fill=None):
    out = [] if out is None else out
    for n in nodes:
        n = dict(n)
        n["_depth"] = depth
        n["_parent"] = parent
        # Fond effectif : celui du bloc, sinon celui qu'il laisse voir de son parent. C'est lui
        # qui décide de la couleur du libellé — un bloc sans fond propre posé sur un panneau
        # sombre est sur du sombre, quoi qu'en dise son absence de `fill`.
        n["_fill"] = n.get("fill") or fill
        out.append(n)
        _flat(n.get("children", []), n.get("id"), depth + 1, out, n["_fill"])
    return out


def _label_ink(fill):
    """Encre lisible sur `fill`. Le plan peint les fonds RÉELS de l'interface relevée : sur un
    panneau de jeu sombre, `var(--ink)` disparaît en thème clair."""
    if not isinstance(fill, str) or not fill.startswith("#"):
        return "var(--ink)"
    h = fill.lstrip("#")
    if len(h) == 3:
        h = "".join(c * 2 for c in h)
    if len(h) < 6:
        return "var(--ink)"
    try:
        r, g, b = (int(h[i:i + 2], 16) for i in (0, 2, 4))
    except ValueError:
        return "var(--ink)"
    return "var(--ink)" if (0.299 * r + 0.587 * g + 0.114 * b) > 140 else "#f2efe9"


def _measure_lines(nodes, scale):
    """Cotes automatiques : gouttieres entre freres alignes."""
    out = []
    for parent in [None] + [n.get("id") for n in _flat(nodes)]:
        sibs = [n for n in _flat(nodes) if n["_parent"] == parent]
        for axis in ("x", "y"):
            i = 0 if axis == "x" else 1
            row = sorted(sibs, key=lambda n: n["box"][i])
            for a, b in zip(row, row[1:]):
                gap = b["box"][i] - a["box"][i + 2]
                if gap <= 0:
                    continue
                if axis == "x":
                    y = (max(a["box"][1], b["box"][1]) + min(a["box"][3], b["box"][3])) / 2
                    out.append(("x", a["box"][2], b["box"][0], y, gap))
                else:
                    x = (max(a["box"][0], b["box"][0]) + min(a["box"][2], b["box"][2])) / 2
                    out.append(("y", a["box"][3], b["box"][1], x, gap))
    return out


def _svg(spec):
    w, h = spec.get("canvas", [800, 600])
    s = float(spec.get("scale", 1))
    pad = 46
    nodes = _flat(spec.get("nodes", []))
    vw, vh = round(w * s + pad * 2, 1), round(h * s + pad * 2, 1)
    parts = [f'<svg viewBox="0 0 {vw} {vh}" width="{vw}" height="{vh}" '
             f'font-family="ui-monospace, Consolas, monospace" font-size="10">']
    parts.append(f'<rect x="{pad}" y="{pad}" width="{w * s}" height="{h * s}" '
                 f'fill="var(--grid)" stroke="var(--rule)" />')

    for n in nodes:
        x0, y0, x1, y1 = [v * s for v in n["box"]]
        stroke, _f, dash = ROLE_STYLE.get(n.get("role", "group"), ROLE_STYLE["group"])
        fill = n.get("fill", "none")
        parts.append(
            f'<rect x="{pad + x0:.1f}" y="{pad + y0:.1f}" width="{x1 - x0:.1f}" '
            f'height="{y1 - y0:.1f}" rx="{float(n.get("radius", 0)) * s:.1f}" '
            f'fill="{_esc(fill)}" stroke="{stroke}" stroke-width="1" '
            f'stroke-dasharray="{dash}" />')
        label = n.get("label") or n.get("id", "")
        if label:
            # Décalé d'un cran par niveau : deux blocs imbriqués dont les bords se touchent
            # (un liseré et son contenu, par exemple) écrivaient sinon leur nom au même
            # endroit, et aucun des deux ne se lisait.
            dy = 11 + n["_depth"] * 11
            parts.append(
                f'<text x="{pad + x0 + 4:.1f}" y="{pad + y0 + dy:.1f}" '
                f'fill="{_label_ink(n.get("_fill"))}" opacity="0.85">{_esc(label)}</text>')

    for axis, a, b, pos, gap in _measure_lines(spec.get("nodes", []), s):
        c = "var(--measure)"
        if axis == "x":
            y = pad + pos * s
            parts.append(f'<line x1="{pad + a * s:.1f}" y1="{y:.1f}" x2="{pad + b * s:.1f}" '
                         f'y2="{y:.1f}" stroke="{c}" stroke-width="1" />')
            parts.append(f'<text x="{pad + (a + b) / 2 * s:.1f}" y="{y - 3:.1f}" fill="{c}" '
                         f'text-anchor="middle">{gap}</text>')
        else:
            x = pad + pos * s
            parts.append(f'<line x1="{x:.1f}" y1="{pad + a * s:.1f}" x2="{x:.1f}" '
                         f'y2="{pad + b * s:.1f}" stroke="{c}" stroke-width="1" />')
            parts.append(f'<text x="{x + 3:.1f}" y="{pad + (a + b) / 2 * s:.1f}" fill="{c}">'
                         f'{gap}</text>')

    parts.append(f'<text x="{pad}" y="{pad - 12}" fill="var(--ink-soft)">'
                 f'{w} × {h} px</text>')
    parts.append("</svg>")
    return "".join(parts)


def _num(v):
    """Nombre lisible : 33.599999999999994 se lit 33.6, 44.0 se lit 44."""
    v = round(float(v), 2)
    return str(int(v)) if v == int(v) else str(v)


def _inventory(spec):
    rows = []
    for n in _flat(spec.get("nodes", [])):
        x0, y0, x1, y1 = n["box"]
        pad = n.get("padding")
        pad_s = ("%s" % pad) if isinstance(pad, str) else (
            " ".join(str(v) for v in pad) if isinstance(pad, list) else "—")
        rows.append(
            "<tr>"
            f'<td class="mono">{"&nbsp;" * (n["_depth"] * 3)}{_esc(n.get("id", ""))}</td>'
            f'<td>{_esc(n.get("label", ""))}</td>'
            f'<td class="mono">{_num(x0)},{_num(y0)}</td>'
            f'<td class="mono">{_num(x1 - x0)}×{_num(y1 - y0)}</td>'
            f'<td class="mono">{_esc(n.get("radius", "—"))}</td>'
            f'<td class="mono">{_esc(pad_s)}</td>'
            f'<td class="note">{_esc(n.get("notes", ""))}</td></tr>')
    head = ("<tr><th>id</th><th>Bloc</th><th>Origine</th><th>Taille</th><th>Rayon</th>"
            "<th>Padding</th><th>Notes</th></tr>")
    return f'<div class="scroll"><table>{head}{"".join(rows)}</table></div>'


def _tokens(spec):
    t = spec.get("tokens", {})
    out = []
    colors = t.get("color", {})
    if colors:
        chips = "".join(
            f'<span class="tok"><span class="dot" style="background:{_esc(v)}"></span>'
            f'{_esc(k)} {_esc(v)}</span>' for k, v in colors.items())
        out.append(f'<section><h2>Couleurs</h2><div class="tokens">{chips}</div></section>')
    space = t.get("space", [])
    if space:
        bars = "".join(f'<div><i style="width:{max(int(v), 2)}px"></i>{v}</div>'
                       for v in space)
        out.append(f'<section><h2>Espacements</h2><div class="scale">{bars}</div></section>')
    radius = t.get("radius", {})
    if radius:
        chips = "".join(f'<span class="tok">{_esc(k)} {_esc(v)}</span>'
                        for k, v in radius.items())
        out.append(f'<section><h2>Rayons</h2><div class="tokens">{chips}</div></section>')
    type_ = t.get("type", [])
    if type_:
        rows = "".join(
            "<tr>"
            f'<td class="mono">{_esc(x.get("name", ""))}</td>'
            f'<td class="mono">{_esc(x.get("size", ""))}</td>'
            f'<td class="mono">{_esc(x.get("weight", ""))}</td>'
            f'<td class="mono">{_esc(x.get("color", ""))}</td>'
            f'<td class="note">{_esc(x.get("usage", ""))}</td></tr>' for x in type_)
        out.append('<section><h2>Typographie</h2><div class="scroll"><table>'
                   "<tr><th>Rôle</th><th>Taille</th><th>Graisse</th><th>Couleur</th>"
                   f"<th>Emploi</th></tr>{rows}</table></div></section>")
    return "".join(out)


def render(spec: dict) -> str:
    title = spec.get("title", "Maquette")
    sub = spec.get("subtitle", "")
    src = spec.get("source", "")
    body = [f"<title>{_esc(title)}</title>", f"<style>{CSS}</style>", '<div class="wrap">',
            f'<header><h1>{_esc(title)}</h1><p>{_esc(sub)}</p>'
            + (f'<p class="src">source : {_esc(src)}</p>' if src else "") + "</header>",
            f'<section><h2>Plan coté</h2><div class="plan">{_svg(spec)}</div></section>',
            f"<section><h2>Blocs</h2>{_inventory(spec)}</section>",
            _tokens(spec)]
    if spec.get("notes"):
        items = "".join(f"<li>{_esc(n)}</li>" for n in spec["notes"])
        body.append(f'<section><h2>Observations</h2><ul class="note">{items}</ul></section>')
    body.append("</div>")
    return "\n".join(body)


if __name__ == "__main__":
    import sys
    print(render(json.loads(open(sys.argv[1], encoding="utf-8").read())))
