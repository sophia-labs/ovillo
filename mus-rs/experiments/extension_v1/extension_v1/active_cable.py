"""Active-cable control model.

This is the deliberately unsuccessful end-to-end baseline that motivated the
exact Clifford operator oracle.  It preserves the architectural commitments:
extended strings in R^4, closed token gestures, a shared active material law,
geometry-derived contacts, persistent field state, and gestural pickup.  On the
complete four-token permutation-parity probe, the recorded small run remained
at chance.  The failure is retained as evidence and as a future research seam.
"""
from __future__ import annotations

import argparse
import itertools
import json
import math
import random
import time
from dataclasses import asdict, dataclass
from pathlib import Path

import torch
from torch import Tensor, nn
import torch.nn.functional as F


@dataclass(frozen=True)
class ActiveCableConfig:
    seed: int = 7
    vocab_size: int = 4
    string_count: int = 8
    nodes_per_string: int = 16
    ambient_dim: int = 4
    channels: int = 8
    reaction_rank: int = 12
    microsteps_per_token: int = 6
    read_paths: int = 2
    read_steps: int = 6
    contact_neighbors: int = 4
    dt: float = 0.045
    train_steps: int = 1200
    learning_rate: float = 3e-3
    weight_decay: float = 2e-5
    cpu_threads: int = 1
    active: bool = True
    gestural_readout: bool = True
    trace_every: int = 50
    batch_size: int = 16
    prefix_supervision: bool = True


def seed_everything(seed: int) -> None:
    random.seed(seed)
    torch.manual_seed(seed)


def permutation_parity(p: tuple[int, ...]) -> int:
    return sum(p[i] > p[j] for i in range(len(p)) for j in range(i + 1, len(p))) % 2


def dataset(vocab_size: int) -> tuple[Tensor, Tensor]:
    perms = list(itertools.permutations(range(vocab_size)))
    x = torch.tensor(perms, dtype=torch.long)
    y = torch.tensor([permutation_parity(p) for p in perms], dtype=torch.long)
    return x, y


def prefix_parity_labels(tokens: Tensor) -> Tensor:
    """Inversion parity of every prefix, shape [batch, sequence]."""
    rows: list[list[int]] = []
    for row in tokens.tolist():
        labels: list[int] = []
        for length in range(1, len(row) + 1):
            prefix = row[:length]
            labels.append(
                sum(prefix[i] > prefix[j] for i in range(length) for j in range(i + 1, length)) % 2
            )
        rows.append(labels)
    return torch.tensor(rows, dtype=torch.long, device=tokens.device)


class ActiveCableMedium(nn.Module):
    """Active cable fields on learned 1-D bodies embedded in R^4.

    No attention, per-site dense weights, or hidden layer stack. A single shared
    constitutive reaction law acts at every material point. Tokens are closed
    paths that inject a local force. Readout samples residual fields along
    learned closed paths.
    """

    def __init__(self, cfg: ActiveCableConfig) -> None:
        super().__init__()
        self.cfg = cfg
        S, M, D, C, V = (
            cfg.string_count,
            cfg.nodes_per_string,
            cfg.ambient_dim,
            cfg.channels,
            cfg.vocab_size,
        )

        # Curves: each neuron is an extended 1-D material body.
        self.curve_base = nn.Parameter(torch.randn(S, 1, D) * 0.38)
        self.curve_increments = nn.Parameter(torch.randn(S, M - 1, D) * 0.11)

        # Closed cubic Bezier gestures p0=0 -> p1 -> p2 -> p3=0.
        self.gesture_controls = nn.Parameter(torch.randn(V, 2, D) * 0.75)
        self.gesture_amplitude = nn.Parameter(torch.randn(V, C) * 0.38)
        self.gesture_velocity_map = nn.Parameter(torch.randn(D, C) * 0.16)
        self.raw_gesture_width = nn.Parameter(torch.tensor(-0.25))

        # Shared active material law. The quadratic reaction is low-rank and
        # identical at every material point; it is a constitutive tensor, not a
        # per-site MLP.
        self.raw_diffusion = nn.Parameter(torch.full((C,), -0.1))
        self.raw_contact = nn.Parameter(torch.full((C,), -0.7))
        self.raw_damping = nn.Parameter(torch.full((C,), -1.0))
        self.raw_recovery_rate = nn.Parameter(torch.full((C,), -2.0))
        self.raw_recovery_gain = nn.Parameter(torch.full((C,), -0.3))
        self.raw_cubic = nn.Parameter(torch.full((C,), -0.7))
        self.raw_excitable_gain = nn.Parameter(torch.full((C,), -0.1))
        self.raw_contact_width = nn.Parameter(torch.tensor(-0.2))

        self.raw_skew = nn.Parameter(torch.randn(C, C) * 0.05)
        R = cfg.reaction_rank
        self.reaction_u = nn.Parameter(torch.randn(C, R) / math.sqrt(C))
        self.reaction_v = nn.Parameter(torch.randn(C, R) / math.sqrt(C))
        self.memory_v = nn.Parameter(torch.randn(C, R) / math.sqrt(C))
        self.recovery_v = nn.Parameter(torch.randn(C, R) / math.sqrt(C))
        self.reaction_out = nn.Parameter(torch.randn(R, C) * 0.08)
        self.self_reaction_out = nn.Parameter(torch.randn(R, C) * 0.035)
        self.raw_reaction_scale = nn.Parameter(torch.tensor(-0.5))

        # Slow material memory modulates excitability locally.
        self.raw_memory_rate = nn.Parameter(torch.full((C,), -2.8))
        self.raw_memory_gain = nn.Parameter(torch.full((C,), -0.4))

        # Learned closed read gestures. A conventional linear map only decodes
        # samples gathered along the physical read paths.
        self.read_controls = nn.Parameter(
            torch.randn(cfg.read_paths, 2, D) * 0.65
        )
        self.raw_read_width = nn.Parameter(torch.tensor(-0.1))
        if cfg.gestural_readout:
            feature_dim = cfg.read_paths * cfg.read_steps * C * 3
        else:
            feature_dim = cfg.read_paths * C * 3
        self.readout = nn.Linear(feature_dim, 2)

    def geometry(self) -> Tensor:
        inc = torch.tanh(self.curve_increments) * 0.20
        return torch.cat(
            [self.curve_base, self.curve_base + torch.cumsum(inc, dim=1)], dim=1
        )

    @staticmethod
    def bezier(controls: Tensor, phase: Tensor) -> tuple[Tensor, Tensor]:
        """Closed cubic Bezier point and tangent.

        controls: [..., 2, D]; phase broadcastable to leading dims.
        """
        p1, p2 = controls[..., 0, :], controls[..., 1, :]
        p0 = torch.zeros_like(p1)
        p3 = p0
        u = phase[..., None]
        one = 1.0 - u
        point = one**3 * p0 + 3 * one**2 * u * p1 + 3 * one * u**2 * p2 + u**3 * p3
        tangent = (
            3 * one**2 * (p1 - p0)
            + 6 * one * u * (p2 - p1)
            + 3 * u**2 * (p3 - p2)
        )
        return point, tangent

    def material_laplacian(self, x: Tensor) -> Tensor:
        c = self.cfg
        B, N, C = x.shape
        y = x.view(B, c.string_count, c.nodes_per_string, C)
        lap = torch.zeros_like(y)
        lap[:, :, 1:-1] = y[:, :, 2:] - 2 * y[:, :, 1:-1] + y[:, :, :-2]
        lap[:, :, 0] = y[:, :, 1] - y[:, :, 0]
        lap[:, :, -1] = y[:, :, -2] - y[:, :, -1]
        return lap.view_as(x)

    def contact_operator(self, points: Tensor) -> Tensor:
        """Symmetric sparse, geometry-derived cross-string conductance."""
        c = self.cfg
        N = points.shape[0]
        d2 = torch.cdist(points, points).square()
        sid = torch.arange(N, device=points.device) // c.nodes_per_string
        forbidden = (sid[:, None] == sid[None, :]) | torch.eye(
            N, dtype=torch.bool, device=points.device
        )
        d2_masked = d2.masked_fill(forbidden, float("inf"))
        k = min(c.contact_neighbors, max(1, N - c.nodes_per_string))
        _, idx = torch.topk(d2_masked, k=k, dim=-1, largest=False)
        mask = torch.zeros_like(d2, dtype=torch.bool)
        mask.scatter_(1, idx, True)
        mask = mask | mask.T
        width = F.softplus(self.raw_contact_width) + 0.18
        W = torch.exp(-d2 / (2 * width.square())) * mask
        W = 0.5 * (W + W.T)
        # Symmetric normalization keeps the contact scale controlled.
        degree = W.sum(-1).clamp_min(1e-6)
        inv = degree.rsqrt()
        return inv[:, None] * W * inv[None, :]

    def reaction(self, v: Tensor, recovery: Tensor, memory: Tensor) -> Tensor:
        """Shared local active law with fast/slow multiplicative interaction."""
        skew = self.raw_skew - self.raw_skew.T
        rotational = v @ skew
        if not self.cfg.active:
            return 0.15 * rotational
        fast = torch.tanh(v @ self.reaction_u)
        slow = torch.tanh(memory @ self.memory_v + recovery @ self.recovery_v)
        gated = (fast * slow) @ self.reaction_out
        self_term = (fast * torch.tanh(v @ self.reaction_v)) @ self.self_reaction_out
        scale = F.softplus(self.raw_reaction_scale) + 0.05
        return rotational + scale * (gated + 0.4 * self_term)

    def step(
        self,
        v: Tensor,
        recovery: Tensor,
        memory: Tensor,
        forcing: Tensor,
        contact: Tensor,
    ) -> tuple[Tensor, Tensor, Tensor, dict[str, Tensor]]:
        c = self.cfg
        diffusion = F.softplus(self.raw_diffusion) + 0.015
        contact_strength = F.softplus(self.raw_contact) + 0.01
        damping = F.softplus(self.raw_damping) + 0.015
        recovery_rate = torch.sigmoid(self.raw_recovery_rate) * 1.20 + 0.04
        recovery_gain = F.softplus(self.raw_recovery_gain) + 0.05
        cubic = F.softplus(self.raw_cubic) + 0.04
        excitable = F.softplus(self.raw_excitable_gain) + 0.08
        memory_rate = torch.sigmoid(self.raw_memory_rate) * 0.80 + 0.02
        memory_gain = F.softplus(self.raw_memory_gain) + 0.02

        lap = self.material_laplacian(v)
        contact_mix = contact @ v
        contact_lap = contact_mix - v
        internal = self.reaction(v, recovery, memory)

        # Active-cable / FitzHugh-Nagumo-inspired shared constitutive law.
        dv = (
            diffusion * lap
            + contact_strength * contact_lap
            + excitable * v
            - cubic * v.pow(3)
            - recovery_gain * recovery
            - damping * v
            + internal
            + forcing
        )
        drecovery = recovery_rate * (v - recovery)
        event = torch.sigmoid(4.0 * (v - 0.35))
        dmemory = memory_rate * (event - memory) + 0.55 * torch.tanh(forcing)

        dt = c.dt
        # Bounded semi-implicit update: enough active dynamics to compute,
        # bounded enough for long unrolls on CPU.
        recovery_next = recovery + dt * drecovery
        memory_next = memory + dt * dmemory
        v_next = v + dt * dv
        v_next = 2.5 * torch.tanh(v_next / 2.5)

        # Approximate audit terms. This is a numerical monitor, not an exact
        # port-Hamiltonian proof.
        audit = {
            "gesture_work": dt * (v * forcing).sum(dim=(1, 2)),
            "damping_loss": dt * (damping * v.square()).sum(dim=(1, 2)),
            "reaction_work": dt * (v * internal).sum(dim=(1, 2)),
        }
        return v_next, recovery_next, memory_next, audit

    def forcing(self, token: Tensor, phase: float, points: Tensor) -> Tensor:
        B = token.shape[0]
        controls = self.gesture_controls[token]
        u = torch.full((B,), phase, dtype=points.dtype, device=points.device)
        point, tangent = self.bezier(controls, u)
        d2 = (points[None, :, :] - point[:, None, :]).square().sum(-1)
        width = F.softplus(self.raw_gesture_width) + 0.20
        kernel = torch.exp(-d2 / (2 * width.square()))
        amp = self.gesture_amplitude[token] + tangent @ self.gesture_velocity_map
        return kernel[:, :, None] * amp[:, None, :]

    def sample_read_paths(
        self, v: Tensor, recovery: Tensor, memory: Tensor, points: Tensor
    ) -> Tensor:
        c = self.cfg
        samples: list[Tensor] = []
        width = F.softplus(self.raw_read_width) + 0.20
        if c.gestural_readout:
            phases = [(j + 0.5) / c.read_steps for j in range(c.read_steps)]
        else:
            phases = [0.5]
        for p in range(c.read_paths):
            controls = self.read_controls[p : p + 1]
            for phase in phases:
                u = torch.tensor([phase], dtype=points.dtype, device=points.device)
                point, _ = self.bezier(controls, u)
                d2 = (points - point[0]).square().sum(-1)
                kernel = torch.softmax(-d2 / (2 * width.square()), dim=0)
                samples.extend(
                    [
                        (kernel[None, :, None] * v).sum(dim=1),
                        (kernel[None, :, None] * recovery).sum(dim=1),
                        (kernel[None, :, None] * memory).sum(dim=1),
                    ]
                )
        return torch.cat(samples, dim=-1)

    def forward(self, tokens: Tensor, *, audit: bool = False, return_prefix: bool = False):
        c = self.cfg
        B, L = tokens.shape
        geometry = self.geometry()
        points = geometry.reshape(-1, c.ambient_dim)
        N = points.shape[0]
        contact = self.contact_operator(points)

        v = torch.zeros(B, N, c.channels, dtype=points.dtype, device=points.device)
        recovery = torch.zeros_like(v)
        memory = torch.zeros_like(v)
        e0 = 0.5 * (v.square() + recovery.square()).sum(dim=(1, 2))
        totals = {"gesture_work": torch.zeros(B, device=points.device),
                  "damping_loss": torch.zeros(B, device=points.device),
                  "reaction_work": torch.zeros(B, device=points.device)}

        prefix_outputs: list[Tensor] = []
        for i in range(L):
            token = tokens[:, i]
            for m in range(c.microsteps_per_token):
                phase = (m + 0.5) / c.microsteps_per_token
                force = self.forcing(token, phase, points)
                v, recovery, memory, terms = self.step(v, recovery, memory, force, contact)
                for name, value in terms.items():
                    totals[name] = totals[name] + value
            if return_prefix:
                prefix_features = self.sample_read_paths(v, recovery, memory, points)
                prefix_outputs.append(self.readout(prefix_features))

        features = self.sample_read_paths(v, recovery, memory, points)
        logits = self.readout(features)
        if not audit:
            if return_prefix:
                return torch.stack(prefix_outputs, dim=1)
            return logits
        e1 = 0.5 * (v.square() + recovery.square()).sum(dim=(1, 2))
        return logits, {
            **totals,
            "initial_energy": e0,
            "final_energy": e1,
            "max_abs_state": v.abs().amax(dim=(1, 2)),
            "geometry_length": (geometry[:, 1:] - geometry[:, :-1]).norm(dim=-1).sum(),
            "contact_nonzero": (contact > 0).sum(),
        }

    def regularization(self) -> Tensor:
        geometry = self.geometry()
        segments = geometry[:, 1:] - geometry[:, :-1]
        lengths = segments.norm(dim=-1)
        length_reg = (lengths - 0.18).square().mean()
        curvature = segments[:, 1:] - segments[:, :-1]
        curvature_reg = curvature.square().mean()
        controls = self.gesture_controls
        gesture_reg = 0.002 * controls.square().mean()
        return 0.2 * length_reg + 0.03 * curvature_reg + gesture_reg


def train_active_cable(cfg: ActiveCableConfig, output: Path | None = None) -> dict[str, object]:
    seed_everything(cfg.seed)
    torch.set_num_threads(cfg.cpu_threads)
    x, y = dataset(cfg.vocab_size)
    model = ActiveCableMedium(cfg)
    opt = torch.optim.AdamW(
        model.parameters(), lr=cfg.learning_rate, weight_decay=cfg.weight_decay
    )
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(opt, T_max=cfg.train_steps)

    best_acc = 0.0
    best_step = 0
    best_state: dict[str, Tensor] | None = None
    trace: list[dict[str, float | int]] = []
    start = time.perf_counter()

    prefix_y = prefix_parity_labels(x)
    for step in range(1, cfg.train_steps + 1):
        sample = torch.randint(0, x.shape[0], (cfg.batch_size,))
        xb = x[sample]
        yb = y[sample]
        if cfg.prefix_supervision:
            prefix_logits = model(xb, return_prefix=True)
            final_loss = F.cross_entropy(prefix_logits[:, -1], yb)
            # Dense supervision teaches the physical state transition itself.
            # It is annealed away so the final phase trains only the endpoint.
            aux = 0.0
            for prefix_index, weight in ((1, 0.35), (2, 0.65)):
                aux = aux + weight * F.cross_entropy(
                    prefix_logits[:, prefix_index], prefix_y[sample, prefix_index]
                )
            anneal = max(0.0, 1.0 - step / max(1.0, 0.72 * cfg.train_steps))
            loss = final_loss + anneal * aux + model.regularization()
        else:
            logits = model(xb)
            loss = F.cross_entropy(logits, yb) + model.regularization()
        if not torch.isfinite(loss):
            raise RuntimeError("non-finite loss")
        opt.zero_grad(set_to_none=True)
        loss.backward()
        nn.utils.clip_grad_norm_(model.parameters(), 2.0)
        opt.step()
        scheduler.step()

        if step == 1 or step % cfg.trace_every == 0 or step == cfg.train_steps:
            with torch.no_grad():
                logits_eval, audit = model(x, audit=True)
                pred = logits_eval.argmax(-1)
                acc = float((pred == y).float().mean())
                margin = float((logits_eval[:, 1] - logits_eval[:, 0]).abs().mean())
                max_state = float(audit["max_abs_state"].max())
                energy = float(audit["final_energy"].mean())
            row = {
                "step": step,
                "loss": float(loss.detach()),
                "accuracy": acc,
                "mean_abs_margin": margin,
                "mean_final_energy": energy,
                "max_abs_state": max_state,
            }
            trace.append(row)
            print(json.dumps(row, sort_keys=True))
            if acc > best_acc:
                best_acc, best_step = acc, step
                best_state = {k: v.detach().cpu().clone() for k, v in model.state_dict().items()}
            if acc == 1.0 and step >= 100:
                # Require persistence for one additional report interval.
                if len(trace) >= 2 and trace[-2]["accuracy"] == 1.0:
                    break

    elapsed = time.perf_counter() - start
    if best_state is not None:
        model.load_state_dict(best_state)
    with torch.no_grad():
        logits, audit = model(x, audit=True)
        pred = logits.argmax(-1)
        final_acc = float((pred == y).float().mean())
        confusion = torch.zeros(2, 2, dtype=torch.int64)
        for truth, guess in zip(y, pred):
            confusion[truth, guess] += 1
        final_audit = {
            k: (float(v.mean()) if v.ndim else float(v))
            for k, v in audit.items()
        }

    result: dict[str, object] = {
        "model": "extension-v1-active-cable",
        "config": asdict(cfg),
        "task": "permutation_parity",
        "dataset_size": int(x.shape[0]),
        "all_examples_same_token_multiset": True,
        "all_token_gestures_closed": True,
        "best_accuracy": best_acc,
        "best_step": best_step,
        "restored_best_accuracy": final_acc,
        "confusion": confusion.tolist(),
        "parameter_count": sum(p.numel() for p in model.parameters()),
        "elapsed_seconds": elapsed,
        "trace": trace,
        "audit_mean": final_audit,
        "claim_discipline": {
            "audit_is_exact_energy_ledger": False,
            "result_is_algorithmic_generalization": False,
            "result_is_full_dataset_architecture_probe": True,
        },
    }
    if output:
        output.mkdir(parents=True, exist_ok=True)
        (output / "result.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
        torch.save({"config": asdict(cfg), "state_dict": model.state_dict()}, output / "model.pt")
    return result


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--steps", type=int, default=1200)
    p.add_argument("--seed", type=int, default=7)
    p.add_argument("--active", action=argparse.BooleanOptionalAction, default=True)
    p.add_argument("--gestural-readout", action=argparse.BooleanOptionalAction, default=True)
    p.add_argument("--strings", type=int, default=8)
    p.add_argument("--nodes", type=int, default=16)
    p.add_argument("--channels", type=int, default=8)
    p.add_argument("--microsteps", type=int, default=6)
    p.add_argument("--lr", type=float, default=3e-3)
    p.add_argument("--cpu-threads", type=int, default=1)
    p.add_argument("--batch-size", type=int, default=16)
    p.add_argument("--prefix-supervision", action=argparse.BooleanOptionalAction, default=True)
    p.add_argument("--output", type=Path)
    args = p.parse_args()
    cfg = ActiveCableConfig(
        seed=args.seed,
        train_steps=args.steps,
        active=args.active,
        gestural_readout=args.gestural_readout,
        string_count=args.strings,
        nodes_per_string=args.nodes,
        channels=args.channels,
        microsteps_per_token=args.microsteps,
        learning_rate=args.lr,
        cpu_threads=args.cpu_threads,
        batch_size=args.batch_size,
        prefix_supervision=args.prefix_supervision,
    )
    result = train_active_cable(cfg, args.output)
    print(json.dumps({k: v for k, v in result.items() if k != "trace"}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
