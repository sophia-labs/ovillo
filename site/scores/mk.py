"""Shared bits for writing MUS-T scores: a bar-length check per voice."""
import re
UNIT = {"w": 32, "h": 16, "q": 8, "e": 4, "s": 2, "t": 1}

def dur32(tok):
    """Length of one token in 32nds (handles dots, ties, triplets, chords)."""
    if tok in ("-",) or tok.startswith("{"):
        return 0
    t = re.sub(r"\[.*?\]", "", tok)
    if t.startswith("<"):
        t = t[t.index(">") + 1:].split("@")[0]
    else:
        t = t.split("@")[0]
        t = t.lstrip("R")
        t = re.sub(r"^[A-G][#b]?-?\d", "", t)
    total = 0.0
    for part in t.split("~"):
        m = re.fullmatch(r"([whqest])(\.*)(3)?", part)
        assert m, f"cannot read duration in {tok!r} ({part!r})"
        base = UNIT[m.group(1)]
        v, add = base, base
        for _ in m.group(2):
            add /= 2; v += add
        if m.group(3):
            v = v * 2 / 3
        total += v
    return total

def bar(n, voices, meta="", beats32=None):
    """voices: dict name -> list of tokens (or '-'). Checks each voice fills the bar."""
    parts = []
    for name, toks in voices.items():
        if toks == "-":
            parts.append(f"{name}=-"); continue
        if beats32 is not None:
            s = sum(dur32(t) for t in toks)
            assert abs(s - beats32) < 1e-6, f"bar {n} {name}: {s} != {beats32}  {toks}"
        parts.append(f"{name}=" + " ".join(toks))
    m = f" [{meta}]" if meta else ""
    return f"bar {n}{m}: " + "  ".join(parts)
