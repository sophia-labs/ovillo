"""Ariadne inverse-geometry reference package."""

from .core import (
    Edge, FloatMatrix, Gate, Graph, CompileCertificate,
    ThermodynamicCertificate, OptimizedTreeCertificate,
    canonical_edge, givens, skew_generator, schedule_matrix, random_so,
    compile_so_on_tree, thermodynamic_schedule, optimize_tree_compilation,
    lie_closure_rank, graph_rotation_lie_rank, graph_transport_lie_rank,
)
from .closed_loop import (
    ClosedLoopCertificate, ExactClosedLoopCertificate,
    balanced_commutator_angle, compile_closed_loop_so3,
    compile_exact_closed_loop_so3,
)
from .work import (
    PumpBoundCertificate, WorkOrderCertificate, spectral_log_radius,
    volume_neutral_anisotropy, verify_pump_bound, verify_factor_order_work,
)
from .topology import (
    TopologicalCertificate, PropagatorCompileCertificate,
    NonAbelianBundleCertificate, rice_mele_hamiltonian, chern_number_fhs,
    polarization_winding, realified_schrodinger_generator,
    verify_topological_pump, compile_skew_propagator, grassmann_frame,
    grassmann_hamiltonian, rectangular_parameter_loop, wilson_loop,
    verify_nonabelian_bundle,
)
from .protected import (
    Dirac4DParameters, SecondChernEstimate,
    KatoLoopCertificate, KatoLoop,
    AdiabaticRunCertificate, AdiabaticConvergenceCertificate,
    AcousticLoweringCertificate, AcousticLowering,
    Z3PumpCertificate, Z3PumpOperators,
    ProtectedPerturbationSummary, ProtectedNonAbelianCertificate,
    ProtectedNonAbelianExperiment,
    clifford_gammas, dirac4d_hamiltonian, occupied_projector,
    analytic_second_chern, second_chern_degree, sampled_homotopy_gap,
    wrap_torus, torus_rectangle_loop, torus_smooth_loop,
    kato_direct_rotation, kato_wilson_loop,
    simulate_adiabatic_loop, verify_adiabatic_convergence,
    realify_unitary, lower_unitary_to_acoustic_graph,
    z3_pump_operators, verify_protected_nonabelian_instrument,
)

__all__ = [name for name in globals() if not name.startswith("_")]
