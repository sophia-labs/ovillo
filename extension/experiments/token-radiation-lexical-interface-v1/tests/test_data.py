from token_radiation_lexical_interface_v1.data import collate_bytes, collate_events
from token_radiation_lexical_interface_v1.world import ByteCodec, generate_example


def test_batches_align_byte_and_event_targets() -> None:
    examples = [
        generate_example(
            latent_seed=1000 + index,
            surface_seed=2000 + index,
            event_count=4,
        )
        for index in range(3)
    ]
    codec = ByteCodec()
    byte_batch = collate_bytes(examples, codec)
    event_batch = collate_events(examples)
    assert byte_batch.ids.shape[0] == 3
    assert event_batch.operator_ids.shape == (3, 4)
    assert event_batch.next_operator.shape == (3, 5)
    assert int((byte_batch.operator >= 0).sum()) == 12
