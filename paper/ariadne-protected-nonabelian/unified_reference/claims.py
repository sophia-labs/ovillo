"""Typed epistemic claim lattice for unified-instrument receipts."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Mapping


class EvidenceClass(str, Enum):
    THEOREM = "theorem"
    VALIDATED_NUMERICS = "validated_numerics"
    REFERENCE_ORACLE = "reference_oracle"
    PRODUCTION_MEASUREMENT = "production_measurement"
    LISTENER_EVIDENCE = "listener_evidence"


class ClaimLevel(str, Enum):
    ORDER_DEPENDENT = "order-dependent"
    CLOSED_CONTROL = "closed-control"
    GEOMETRIC = "geometric"
    GAPPED_FAMILY = "gapped-family"
    TOPOLOGICAL_BUNDLE = "topological-bundle"
    ADIABATIC_REALIZATION = "adiabatic-realization"
    PROTECTED_ACOUSTIC = "protected-acoustic"
    FAULT_TOLERANT = "fault-tolerant"


_LEVEL_ORDER = (
    ClaimLevel.ORDER_DEPENDENT,
    ClaimLevel.CLOSED_CONTROL,
    ClaimLevel.GEOMETRIC,
    ClaimLevel.GAPPED_FAMILY,
    ClaimLevel.TOPOLOGICAL_BUNDLE,
    ClaimLevel.ADIABATIC_REALIZATION,
    ClaimLevel.PROTECTED_ACOUSTIC,
    ClaimLevel.FAULT_TOLERANT,
)


@dataclass(frozen=True)
class ClaimEvidence:
    same_endpoint_paths: bool
    persistent_state_order_defect: bool
    projector_transport: bool
    globally_isolated_rank_two_family: bool
    nonzero_second_chern: bool
    finite_time_replay: bool
    theorem_budget_accepted: bool
    lawful_sparse_lowering: bool
    protected_modes_acoustically_participating: bool
    passive_radiation_from_state: bool
    one_whole_instrument_ledger: bool
    production_cross_replay: bool
    listener_discrimination: bool
    correction_or_fault_tolerance: bool


@dataclass(frozen=True)
class ClaimDecision:
    maximum_reference_claim: ClaimLevel
    maximum_production_claim: ClaimLevel | None
    evidence_classes: tuple[EvidenceClass, ...]
    promoted: tuple[ClaimLevel, ...]
    refused: Mapping[str, str]
    scope: str


def evaluate_claims(evidence: ClaimEvidence) -> ClaimDecision:
    promoted: list[ClaimLevel] = []
    refused: dict[str, str] = {}

    if evidence.persistent_state_order_defect:
        promoted.append(ClaimLevel.ORDER_DEPENDENT)
    else:
        refused[ClaimLevel.ORDER_DEPENDENT.value] = "no persistent AB/BA state defect"

    if evidence.same_endpoint_paths and evidence.persistent_state_order_defect:
        promoted.append(ClaimLevel.CLOSED_CONTROL)
    else:
        refused[ClaimLevel.CLOSED_CONTROL.value] = "same-endpoint path/state conditions incomplete"

    if evidence.projector_transport and ClaimLevel.CLOSED_CONTROL in promoted:
        promoted.append(ClaimLevel.GEOMETRIC)
    else:
        refused[ClaimLevel.GEOMETRIC.value] = "no certified projector/Kato transport"

    if evidence.globally_isolated_rank_two_family:
        promoted.append(ClaimLevel.GAPPED_FAMILY)
    else:
        refused[ClaimLevel.GAPPED_FAMILY.value] = "global rank-two gap not established"

    if evidence.nonzero_second_chern and ClaimLevel.GAPPED_FAMILY in promoted:
        promoted.append(ClaimLevel.TOPOLOGICAL_BUNDLE)
    else:
        refused[ClaimLevel.TOPOLOGICAL_BUNDLE.value] = "nonzero invariant or global gap absent"

    if evidence.finite_time_replay and evidence.theorem_budget_accepted:
        promoted.append(ClaimLevel.ADIABATIC_REALIZATION)
    else:
        refused[ClaimLevel.ADIABATIC_REALIZATION.value] = (
            "finite-time replay exists, but the requested rigorous duration/error budget is not accepted"
            if evidence.finite_time_replay
            else "no finite-time realization"
        )

    protected_acoustic_conditions = (
        evidence.protected_modes_acoustically_participating
        and evidence.passive_radiation_from_state
        and evidence.one_whole_instrument_ledger
        and evidence.lawful_sparse_lowering
        and ClaimLevel.TOPOLOGICAL_BUNDLE in promoted
    )
    if protected_acoustic_conditions:
        promoted.append(ClaimLevel.PROTECTED_ACOUSTIC)
    else:
        refused[ClaimLevel.PROTECTED_ACOUSTIC.value] = (
            "coupled acoustic participation, passive state radiation, ledger, or topology is incomplete"
        )

    if evidence.correction_or_fault_tolerance:
        promoted.append(ClaimLevel.FAULT_TOLERANT)
    else:
        refused[ClaimLevel.FAULT_TOLERANT.value] = "no error-correcting or fault-tolerance mechanism"

    maximum_reference = max(promoted, key=_LEVEL_ORDER.index)
    maximum_production: ClaimLevel | None = None
    if evidence.production_cross_replay:
        maximum_production = maximum_reference
    else:
        refused["production-promotion"] = (
            "Python reference is not yet independently replayed by production Rust/Weave and Lean"
        )

    classes = [EvidenceClass.THEOREM, EvidenceClass.VALIDATED_NUMERICS, EvidenceClass.REFERENCE_ORACLE]
    if evidence.production_cross_replay:
        classes.append(EvidenceClass.PRODUCTION_MEASUREMENT)
    if evidence.listener_discrimination:
        classes.append(EvidenceClass.LISTENER_EVIDENCE)
    else:
        refused["listener-evidence"] = "audible files exist, but no blinded listener result"

    return ClaimDecision(
        maximum_reference_claim=maximum_reference,
        maximum_production_claim=maximum_production,
        evidence_classes=tuple(classes),
        promoted=tuple(promoted),
        refused=refused,
        scope=(
            "The reference model may carry a protected-acoustic oracle claim while production "
            "remains unpromoted until independent Rust/Weave replay and formal checks pass."
        ),
    )
