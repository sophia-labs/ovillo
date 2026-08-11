import json
from pathlib import Path
import unittest

import numpy as np

from mus_analysis.garganta_anatomy import (
    AnatomyGenome,
    EvolutionConfig,
    _AnatomyObjective,
    AcousticState,
    ReferenceAnalysis,
    area_function_cm2,
    build_control_bank,
    isophonemy_distance,
    modeled_envelope_bank,
    modeled_formants,
    reflection_coefficients,
    transfer_response,
)


class GargantaGeometryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.frequency_grid = np.geomspace(90.0, 5200.0, 128)
        self.center = AnatomyGenome.from_unit_cube(np.full(10, 0.5))

    def test_uniform_junctions_have_zero_reflection(self) -> None:
        np.testing.assert_allclose(
            reflection_coefficients([2.0, 2.0, 2.0, 2.0]),
            np.zeros(3),
            atol=1e-12,
        )

    def test_area_cone_is_positive_and_near_closure_is_finite(self) -> None:
        controls = np.array([0.0, 0.0, 0.0, 0.0, 1.0])
        areas = area_function_cm2(self.center, controls)
        self.assertGreater(float(areas.min()), 0.0)
        response = transfer_response(self.center, controls, self.frequency_grid)
        self.assertTrue(np.all(np.isfinite(response)))

    def test_center_anatomy_has_human_scale_low_formant(self) -> None:
        formants = modeled_formants(self.center, np.zeros(5), self.frequency_grid)
        self.assertGreaterEqual(len(formants), 3)
        # A 17.25 cm quarter-wave tube is about 497 Hz; shaping moves it, but
        # it should remain in the same physical neighbourhood.
        self.assertGreater(formants[0], 350.0)
        self.assertLess(formants[0], 700.0)

    def test_isophonemy_is_zero_for_identical_anatomy(self) -> None:
        bank = build_control_bank(32, 7)
        distance = isophonemy_distance(
            self.center,
            self.center,
            self.frequency_grid,
            control_bank=bank,
        )
        self.assertLess(distance, 1e-10)


class GargantaBilevelTests(unittest.TestCase):
    def test_matching_anatomy_beats_far_length_on_synthetic_states(self) -> None:
        config = EvolutionConfig(
            frequency_bins=96,
            representative_states=6,
            control_bank_size=32,
            generations=1,
            population=8,
            local_polish=False,
        )
        frequencies = np.geomspace(
            config.frequency_min_hz,
            config.frequency_max_hz,
            config.frequency_bins,
        )
        true_unit = np.array(
            [0.46, 0.58, 0.43, 0.55, 0.57, 0.44, 0.63, 0.39, 0.52, 0.28]
        )
        true_anatomy = AnatomyGenome.from_unit_cube(true_unit)
        controls = np.array(
            [
                [-0.2, -0.65, 0.20, 0.15, 0.0],
                [0.55, 0.50, -0.15, -0.45, 0.0],
                [-0.35, 0.15, 0.72, 0.55, 0.0],
                [0.10, -0.20, -0.45, -0.10, 0.35],
                [0.35, 0.30, 0.10, 0.70, -0.25],
                [-0.10, 0.65, -0.45, -0.55, 0.15],
            ]
        )
        envelopes = modeled_envelope_bank(true_anatomy, controls, frequencies)
        states = tuple(
            AcousticState(
                state_id=f"state-{index}",
                time_seconds=0.1 * index,
                normalized_envelope=tuple(float(value) for value in envelope),
                formants_hz=modeled_formants(
                    true_anatomy, controls[index], frequencies
                ),
                f0_hz=130.0,
                periodicity=0.9,
                rms_dbfs=-12.0,
                source_tilt_db_per_octave=-8.0,
            )
            for index, envelope in enumerate(envelopes)
        )
        analysis = ReferenceAnalysis(
            sample_rate=16_000,
            duration_seconds=2.0,
            frequency_grid_hz=tuple(float(value) for value in frequencies),
            states=states,
            training_state_ids=tuple(state.state_id for state in states[:4]),
            holdout_state_ids=tuple(state.state_id for state in states[4:]),
            voiced_frame_count=100,
            total_frame_count=120,
            median_f0_hz=130.0,
            f0_iqr_hz=4.0,
            jitter_local=0.01,
            median_periodicity=0.9,
            median_source_tilt_db_per_octave=-8.0,
        )
        bank = np.concatenate([controls, build_control_bank(32, config.seed)], axis=0)
        objective = _AnatomyObjective(analysis, config, bank)
        true_score = objective(true_unit)
        far_unit = true_unit.copy()
        far_unit[0] = 0.98  # much longer tract
        far_score = objective(far_unit)
        self.assertLess(true_score.objective_loss, far_score.objective_loss)
        self.assertLess(true_score.holdout_loss, far_score.holdout_loss)

    def test_claim_schema_marks_non_medical_identity_model(self) -> None:
        # This is a contract test against accidental claim inflation. The
        # complete fit object is tested by the CLI smoke test in publication CI.
        schema = json.loads(
            (Path(__file__).parents[1] / "schemas" / "garganta-speaker-throat.schema.json").read_text()
        )
        claim = schema["properties"]["claim"]["properties"]
        self.assertFalse(claim["medical_anatomy_reconstruction"]["const"])
        self.assertTrue(claim["speaker_identity_model"]["const"])


if __name__ == "__main__":
    unittest.main()
