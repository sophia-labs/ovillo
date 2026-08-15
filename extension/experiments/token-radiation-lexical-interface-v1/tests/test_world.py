from token_radiation_lexical_interface_v1.world import (
    apply_operator,
    generate_example,
    rerender_example,
)


def test_rotation_and_reflection_do_not_commute() -> None:
    neutral = (0, 0, 0)
    rotate_after_reflect = apply_operator(apply_operator(neutral, "F"), "R")
    reflect_after_rotate = apply_operator(apply_operator(neutral, "R"), "F")
    assert rotate_after_reflect != reflect_after_rotate


def test_rerender_preserves_latent_operators_but_changes_surface() -> None:
    original = generate_example(
        latent_seed=1007,
        surface_seed=2007,
        event_count=12,
        surface_policy="homograph_heavy",
    )
    alternate = rerender_example(
        original,
        surface_seed=9991,
        surface_policy="counterfactual",
    )
    assert original.latent_operators == alternate.latent_operators
    assert original.initial_state == alternate.initial_state
    assert original.surface != alternate.surface


def test_context_dependent_homograph_exists() -> None:
    found = {}
    for seed in range(1000, 1200):
        example = generate_example(
            latent_seed=seed,
            surface_seed=seed + 50_000,
            event_count=12,
            surface_policy="homograph_heavy",
        )
        for span in example.spans:
            if span.surface == b"ba":
                found.setdefault(span.mode_before, set()).add(span.operator)
    assert 0 in found and 1 in found
    assert found[0] != found[1]
