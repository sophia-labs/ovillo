from __future__ import annotations

import argparse
import json
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np


def load(path: Path):
    return json.loads(path.read_text())


def plot_data(root: Path):
    for candidate in [root / "PLOT-DATA.json", root / "results/PLOT-DATA.json"]:
        if candidate.exists():
            return load(candidate)
    directory = root / "results" / "plot_data"
    if directory.is_dir():
        return {path.stem: load(path) for path in sorted(directory.glob("*.json"))}
    raise FileNotFoundError("plot data was not found under the experiment root")


def save(fig, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fig.tight_layout()
    fig.savefig(path, dpi=180, bbox_inches="tight")
    plt.close(fig)


def atlas_concordance(root: Path, out: Path) -> None:
    data = plot_data(root)["atlas"]["correlations"]
    names = [
        "state_size",
        "state_participation_rank",
        "balanced_participation_rank",
        "spectral_overlap",
        "belief_r2",
        "future_r2",
        "future_modes",
    ]
    labels = ["State size", "State PR", "Balanced PR", "Spectral overlap", "Belief R²", "Future R²", "Future modes"]
    values = [data[n]["within_process_pairwise_concordance"] for n in names]
    fig, ax = plt.subplots(figsize=(9.2, 4.8))
    ax.bar(labels, values)
    ax.axhline(0.5, linewidth=1)
    ax.set_ylim(0.45, 1.0)
    ax.set_ylabel("Within-process pairwise concordance")
    ax.set_title("Predictive alignment ranks models better than raw state size")
    ax.tick_params(axis="x", rotation=30)
    for i, value in enumerate(values):
        ax.text(i, value + 0.008, f"{value:.3f}", ha="center", va="bottom", fontsize=8)
    save(fig, out / "01_atlas_concordance.png")


def atlas_future_scatter(root: Path, out: Path) -> None:
    data = plot_data(root)["atlas"]
    rows = data["models"]
    fig, ax = plt.subplots(figsize=(7.2, 5.2))
    for process in sorted({row["process_name"] for row in rows}):
        group = [row for row in rows if row["process_name"] == process]
        x = [row["future_modes"] for row in group]
        y = [row["excess_bps"] for row in group]
        ax.scatter(x, y, label=process, s=30)
    ax.set_xlabel("Future predictive modes with R² > 0.5")
    ax.set_ylabel("Excess bits per symbol")
    ax.set_title("Useful future-aligned modes predict model order within a process")
    ax.legend(fontsize=7, ncol=2)
    save(fig, out / "02_atlas_future_modes.png")


def projective_initializer(root: Path, out: Path) -> None:
    data = plot_data(root)["projective"]["conditions"]
    order = ["exact", "nmf", "nmf_horizon", "rank_continuation", "horizon", "random"]
    labels = ["Exact", "Observed NMF", "NMF + horizon", "Rank continuation", "Horizon only", "Random"]
    values = [data[name]["mean_excess_bps"] for name in order]
    errors = [data[name]["sample_sd"] for name in order]
    fig, ax = plt.subplots(figsize=(9.2, 4.8))
    ax.bar(labels, values, yerr=errors, capsize=3)
    ax.set_yscale("log")
    ax.set_ylabel("Excess bits per symbol (log scale)")
    ax.set_title("Observable positive factorization enters the near-oracle basin")
    ax.tick_params(axis="x", rotation=25)
    save(fig, out / "03_projective_initializer.png")


def projective_stream(root: Path, out: Path) -> None:
    rows = plot_data(root)["projective"]["stream_sweep"]
    x = np.array([row["stream_length"] for row in rows])
    excess = np.array([row["initial_excess_bps"] for row in rows])
    residual = np.array([row["mean_shift_residual"] for row in rows])
    fig, ax = plt.subplots(figsize=(7.2, 5.0))
    ax.plot(x, excess, marker="o", label="Initial predictive excess BPS")
    ax.plot(x, residual, marker="s", label="Mean shifted-Hankel residual")
    ax.set_xscale("log")
    ax.set_xlabel("Observed stream length")
    ax.set_ylabel("Error")
    ax.set_title("More observed data does not monotonically improve the factorization")
    ax.legend()
    save(fig, out / "04_projective_stream_conditioning.png")


def balanced_falsification(root: Path, out: Path) -> None:
    data = plot_data(root)["balanced"]
    order = ["baseline", "future_aux", "cca", "aux_cca"]
    labels = ["Baseline", "Future auxiliary", "CCA regularizer", "Auxiliary + CCA"]
    excess = [data[name]["mean_excess_bps"] for name in order]
    rank = [data[name]["mean_balanced_rank"] for name in order]
    fig, ax = plt.subplots(figsize=(7.6, 5.2))
    ax.scatter(rank, excess, s=70)
    for x, y, label in zip(rank, excess, labels):
        ax.annotate(label, (x, y), xytext=(5, 4), textcoords="offset points", fontsize=8)
    ax.set_xlabel("Measured balanced participation rank")
    ax.set_ylabel("Excess bits per symbol")
    ax.set_title("Raising generic balanced rank did not improve prediction")
    save(fig, out / "05_balanced_regularizer_falsification.png")


def attention_phase(root: Path, out: Path) -> None:
    rows = plot_data(root)["attention"]
    standard = [r for r in rows if abs(r["collision_noise"] - 1e-3) < 1e-12 and r["seed"] == 7]
    matrix = np.full((4, 6), np.nan)
    for row in standard:
        matrix[row["routes"] - 1, row["heads"] - 1] = row["functional_rank"]
    fig, ax = plt.subplots(figsize=(8.0, 4.9))
    image = ax.imshow(matrix, aspect="auto", origin="lower")
    fig.colorbar(image, ax=ax, label="Learned functional participation rank")
    ax.set_xticks(range(6), labels=range(1, 7))
    ax.set_yticks(range(4), labels=range(1, 5))
    ax.set_xlabel("Declared heads")
    ax.set_ylabel("Independent retrieval routes")
    ax.set_title("Functional head rank follows min(heads, task routing rank)")
    for i in range(matrix.shape[0]):
        for j in range(matrix.shape[1]):
            ax.text(j, i, f"{matrix[i, j]:.2f}", ha="center", va="center", fontsize=7)
    save(fig, out / "06_attention_rank_phase.png")


def collision_timing(root: Path, out: Path) -> None:
    data = plot_data(root)["attention"]
    rows = [r for r in data if r["routes"] == 3 and r["heads"] == 3 and r["emergence_step"] is not None]
    x = np.array([r["collision_noise"] for r in rows])
    y = np.array([r["emergence_step"] for r in rows])
    fig, ax = plt.subplots(figsize=(7.2, 4.8))
    ax.scatter(x, y, s=55)
    ax.set_xscale("log")
    ax.set_xlabel("Initial head-collision noise")
    ax.set_ylabel("Emergence step")
    ax.set_title("Naive log-noise law for symmetry-breaking time is falsified")
    save(fig, out / "07_attention_collision_timing.png")


def transport_gates(root: Path, out: Path) -> None:
    data = plot_data(root)["mixture"]
    full = data["full"]
    commuting = data["commuting"]
    labels = ["Filter task: projective", "Filter task: Clifford", "Area task: projective", "Area task: Clifford"]
    full_values = [*full["obs_gate"], *full["area_gate"]]
    commuting_values = [*commuting["obs_gate"], *commuting["area_gate"]]
    x = np.arange(len(labels))
    width = 0.38
    fig, ax = plt.subplots(figsize=(10.0, 4.8))
    ax.bar(x - width / 2, full_values, width, label="Full noncommuting bank")
    ax.bar(x + width / 2, commuting_values, width, label="Commuting Clifford control")
    ax.set_xticks(x, labels=labels, rotation=25, ha="right")
    ax.set_ylim(0, 1.05)
    ax.set_ylabel("Learned gate weight")
    ax.set_title("Tasks select different transport categories")
    ax.legend()
    save(fig, out / "08_transport_category_gates.png")


def compartment_convergence(root: Path, out: Path) -> None:
    rows = plot_data(root)["compartmental"]
    x = np.array([r["microsteps"] for r in rows])
    transition = np.array([r["transition_frobenius_error"] for r in rows])
    excess = np.array([r["excess_bps"] for r in rows])
    fig, ax = plt.subplots(figsize=(7.2, 5.0))
    ax.loglog(x, transition, marker="o", label="Transition Frobenius error")
    ax.loglog(x, excess, marker="s", label="Predictive excess BPS")
    ax.set_xlabel("Local sparse microsteps")
    ax.set_ylabel("Error (log scale)")
    ax.set_title("Local cube transport converges to the dense positive predictor")
    ax.legend()
    save(fig, out / "09_compartment_convergence.png")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    atlas_concordance(args.root, args.output)
    atlas_future_scatter(args.root, args.output)
    projective_initializer(args.root, args.output)
    projective_stream(args.root, args.output)
    balanced_falsification(args.root, args.output)
    attention_phase(args.root, args.output)
    collision_timing(args.root, args.output)
    transport_gates(args.root, args.output)
    compartment_convergence(args.root, args.output)


if __name__ == "__main__":
    main()
