import torch

from token_radiation_lexical_interface_v1.models import PredictiveOperatorBody


def test_teacher_forcing_and_feedback_use_identical_source_path() -> None:
    torch.manual_seed(3)
    model = PredictiveOperatorBody(sites=8, radiation_fraction=0.04)
    initial = torch.tensor([0, 7, 15])
    operators = torch.tensor(
        [
            [0, 2, 1, 3],
            [3, 0, 1, 2],
            [2, 2, 0, 1],
        ]
    )
    direct = model(initial, operators, loading=True)
    frame = model.start(initial, loading=True)
    states = [frame.state_after]
    emissions = [frame.emission]
    state = frame.state_after
    for position in range(operators.shape[1]):
        frame = model.feedback(state, operators[:, position], loading=True)
        state = frame.state_after
        states.append(state)
        emissions.append(frame.emission)
    assert torch.allclose(
        torch.stack(states, dim=1), direct.trace.states, atol=1.0e-6
    )
    assert torch.allclose(
        torch.stack(emissions, dim=1), direct.trace.emissions, atol=1.0e-6
    )
