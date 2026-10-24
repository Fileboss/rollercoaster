"""Document options of Looping: settings offset -> title, description, values.

Reads the annotated disassembly (asmann.py output) and joins:
  * the option setters (0x140078640..0x14007d8e0): `mov [rsi+OFF], ecx`
    then, per value, a label and `mov [rsi+OFF], N` (value 0 first);
  * the property grid builder (0x1400f0000..0x140102000): a property is a
    title (`lea rdx, L'...'` before the CMFCPropertyGridProperty ctor), a
    description (`lea r9`), then its options (AddOption, 0x14022185c).
They are matched by their set of value labels.

usage: options.py looping_s.asm [--json]
"""
import json, re, sys

LINE = re.compile(r"^\s*([0-9a-f]+):\s+(\S+)\s+(.*)$")
LABEL = re.compile(r"; L'(.*)'$")


def lines(path, lo, hi):
    for l in open(path, encoding="utf-8", errors="replace"):
        m = LINE.match(l)
        if m and lo <= int(m.group(1), 16) < hi:
            yield int(m.group(1), 16), m.group(2), m.group(3)


def setters(path):
    """{offset: [labels by value]}"""
    out = {}
    cur, labels, pending = None, {}, None
    for _, op, args in lines(path, 0x140078640, 0x14007D8E0):
        m = re.match(r"DWORD PTR \[r\w+\+0x([0-9a-f]+)\],(ecx|0x[0-9a-f]+)$", args) if op == "mov" else None
        lab = LABEL.search(args)
        if m:
            off, val = int(m.group(1), 16), m.group(2)
            if val == "ecx":
                if cur is not None and labels:
                    out.setdefault(cur, labels)
                cur, labels, pending = off, {}, 0
            elif off == cur and pending is not None:
                labels[int(val, 16)] = pending_label
        elif lab and cur is not None:
            pending_label = lab.group(1)
            if pending == 0 and 0 not in labels:
                labels[0] = pending_label
    if cur is not None and labels:
        out.setdefault(cur, labels)
    return {k: [v[i] for i in sorted(v)] for k, v in out.items()}


def grid(path):
    """[(title, description, [options])]"""
    props, strings = [], []
    for _, op, args in lines(path, 0x1400F0000, 0x140102000):
        lab = LABEL.search(args)
        if lab and op == "lea":
            strings.append((args.split(",")[0], lab.group(1)))
        if op == "call" and args.startswith("0x1400f30c0"):  # property ctor
            title = next((s for r, s in reversed(strings) if r == "rdx"), None)
            desc = next((s for r, s in reversed(strings) if r == "r9"), "")
            props.append([title, desc, []])
            strings = []
        elif op == "call" and args.startswith("0x14022185c") and props:  # AddOption
            opt = next((s for r, s in reversed(strings) if r == "rdx"), None)
            if opt is not None:
                props[-1][2].append(opt)
            strings = []
    return props


def main():
    path = sys.argv[1]
    s, g = setters(path), grid(path)
    rows = []
    for off, labels in sorted(s.items()):
        match = next((p for p in g if p[2] and set(p[2]) == set(labels)), None)
        rows.append({"offset": off, "title": match and match[0], "description": match and match[1], "values": labels})
    if "--json" in sys.argv:
        print(json.dumps(rows, ensure_ascii=False, indent=1))
    else:
        for r in rows:
            print(f"+{r['offset']:#05x}  {r['title']!s:40}  {r['values']}")


main()
