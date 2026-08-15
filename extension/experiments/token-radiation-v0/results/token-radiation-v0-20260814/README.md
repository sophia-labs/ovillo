# Token Radiation v0 committed construction

This directory preserves the exact deterministic construction artifact produced
for seed `20260814`.

`summary.json` is stored as three byte-preserving shards so the full detailed
ledger remains reviewable in Git. `scripts/verify_committed_result.py` joins the
shards in lexical order, verifies the original `summary.json` SHA-256 from
`manifest.sha256.json`, and then verifies the internal scientific digest.

```text
scientific SHA-256
37481aaa0c437e3c14a33fec94692bde886ae79baaf94599c7d9683a0397cfb0
```

The directory is evidence for one exploratory synthetic-process construction.
It is not a natural-language, tokenizer-independence, or architecture-superiority
result.
