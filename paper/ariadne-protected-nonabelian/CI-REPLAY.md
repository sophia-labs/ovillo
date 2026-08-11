# CI replay note

The protected non-Abelian packet is verified through three independent paths:

1. the Python topological and adiabatic oracle;
2. the Rust sparse-operator replay and exact Z3 pump fixture;
3. the Lean finite pump algebra.

This note intentionally triggers a clean workflow replay after the Rust fixture was formatted and its maximum-difference fold was made Clippy-canonical.
