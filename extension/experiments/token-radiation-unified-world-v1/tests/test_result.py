from token_radiation_unified_world_v1.experiment import ExperimentConfig, run_campaign, verify_result

def test_smoke_result_verifies(tmp_path):
    config = ExperimentConfig(train_steps=1, batch_size=4, train_events=2, test_events=2, sites=8, courses=2, settle_steps=1, replicates=1)
    result = run_campaign(config, tmp_path)
    assert result["decision"]["gpu_scale_warranted"] is False
    verify_result(tmp_path)
