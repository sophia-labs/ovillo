impl LocalMediumWorldBody {
    pub(crate) fn try_new(
        sample_rate_hz: f64,
        spec: LocalMediumBoundarySpec,
    ) -> Result<Self, ConstitutiveBodyError> {
        spec.validate(sample_rate_hz)
            .map_err(|error: PlanError| ConstitutiveBodyError::LocalMedium(error.to_string()))?;
        let radiation_direction = normalize(spec.radiation_direction).ok_or_else(|| {
            ConstitutiveBodyError::LocalMedium("invalid radiation direction".to_owned())
        })?;
        let inputs = spec
            .inputs
            .iter()
            .map(|binding| {
                Ok(CompiledInputBinding {
                    source: binding.source,
                    boundary_direction: normalize(binding.boundary_direction).ok_or_else(|| {
                        ConstitutiveBodyError::LocalMedium(
                            "invalid input boundary direction".to_owned(),
                        )
                    })?,
                    wave_gain: binding.wave_gain,
                    evidence_gain: binding.evidence_gain,
                })
            })
            .collect::<Result<Vec<_>, ConstitutiveBodyError>>()?;
        let outputs = spec
            .outputs
            .iter()
            .map(|binding| {
                Ok(CompiledOutputBinding {
                    wave_weights: binding.wave_weights.clone(),
                    evidence_weights: binding.evidence_weights.clone(),
                    reservoir_weights: binding.reservoir_weights.clone(),
                    boundary_direction: normalize(binding.boundary_direction).ok_or_else(|| {
                        ConstitutiveBodyError::LocalMedium(
                            "invalid output boundary direction".to_owned(),
                        )
                    })?,
                    gain_per_second: binding.gain_per_second,
                })
            })
            .collect::<Result<Vec<_>, ConstitutiveBodyError>>()?;
        let medium = LocalConstitutiveMedium::from_plan(spec.medium.clone())?;
        Ok(Self {
            boundary_reservoir: spec.initial_boundary_reservoir,
            spec,
            medium,
            dt: sample_rate_hz.recip(),
            boundary_momentum: [0.0; MODE_COUNT],
            radiation_direction,
            inputs,
            outputs,
        })
    }

    #[must_use]
    pub(crate) fn state(&self) -> &LocalMediumState {
        self.medium.state()
    }

    #[must_use]
    pub(crate) fn material_stored_energy(&self) -> f64 {
        let medium_material = self
            .medium
            .plan()
            .sites
            .iter()
            .enumerate()
            .map(|(index, site)| {
                0.5 * site.wave_metric * self.medium.state().wave[index].powi(2)
                    + 0.5 * self.medium.state().evidence[index].powi(2)
            })
            .sum::<f64>();
        medium_material + kinetic(&self.boundary_momentum, self.dt)
    }

    #[must_use]
    pub(crate) fn reservoir_energy(&self) -> f64 {
        self.boundary_reservoir + self.medium.state().reservoir.iter().sum::<f64>()
    }

    #[must_use]
    pub(crate) fn total_stored_energy(&self) -> f64 {
        self.material_stored_energy() + self.reservoir_energy()
    }

    #[must_use]
    pub(crate) fn momentum(&self) -> &[f64; MODE_COUNT] {
        &self.boundary_momentum
    }

    pub(crate) fn momentum_mut(&mut self) -> &mut [f64; MODE_COUNT] {
        &mut self.boundary_momentum
    }

    fn source_work_for_input(
        &self,
        input: &LocalMediumInputFrame,
    ) -> Result<f64, LocalMediumError> {
        let mut wave = self.medium.state().wave.clone();
        let mut evidence = self.medium.state().evidence.clone();
        let mut reservoir = self.medium.state().reservoir.clone();
        let mut source_work = 0.0;
        for drive in &input.drives {
            let source = self
                .medium
                .plan()
                .sources
                .get(drive.source)
                .ok_or(LocalMediumError::UnknownSource(drive.source))?;
            let site = source.site;
            let metric = self.medium.plan().sites[site].wave_metric;
            let before_wave = 0.5 * metric * wave[site] * wave[site];
            wave[site] += source.wave_gain * finite(drive.wave);
            source_work += 0.5 * metric * wave[site] * wave[site] - before_wave;

            let before_evidence = 0.5 * evidence[site] * evidence[site];
            evidence[site] += (source.evidence_gain * finite(drive.evidence)).max(0.0);
            source_work += 0.5 * evidence[site] * evidence[site] - before_evidence;

            let requested = (source.work_gain * finite(drive.work_rate)).max(0.0) * self.dt;
            let capacity = self.medium.plan().sites[site].reservoir_capacity;
            let added = requested.min((capacity - reservoir[site]).max(0.0));
            reservoir[site] += added;
            source_work += added;
        }
        Ok(source_work)
    }

    fn port_input(&self) -> LocalMediumInputFrame {
        let drives = self
            .inputs
            .iter()
            .map(|binding| {
                let coordinate = dot(&self.boundary_momentum, &binding.boundary_direction);
                LocalMediumSourceDrive {
                    source: binding.source,
                    wave: binding.wave_gain * coordinate,
                    evidence: (binding.evidence_gain * coordinate).max(0.0),
                    work_rate: 0.0,
                }
            })
            .collect();
        LocalMediumInputFrame { drives }
    }

    fn output_value(&self, binding: &CompiledOutputBinding) -> f64 {
        binding
            .wave_weights
            .iter()
            .zip(&self.medium.state().wave)
            .map(|(weight, value)| weight * value)
            .chain(
                binding
                    .evidence_weights
                    .iter()
                    .zip(&self.medium.state().evidence)
                    .map(|(weight, value)| weight * value),
            )
            .chain(
                binding
                    .reservoir_weights
                    .iter()
                    .zip(&self.medium.state().reservoir)
                    .map(|(weight, value)| weight * value),
            )
            .sum()
    }

    fn check_internal_transfer(&self, work: f64) -> Result<(), ConstitutiveBodyError> {
        if work >= 0.0 {
            if work > self.boundary_reservoir + 1.0e-12 {
                return Err(ConstitutiveBodyError::BoundaryReservoirExhausted);
            }
        } else {
            let returned = -work;
            let room = self.spec.boundary_reservoir_capacity - self.boundary_reservoir;
            if returned > room + 1.0e-12 {
                return Err(ConstitutiveBodyError::BoundaryReservoirOverflow);
            }
        }
        Ok(())
    }

    fn settle_internal_transfer(&mut self, work: f64) {
        if work >= 0.0 {
            self.boundary_reservoir = (self.boundary_reservoir - work).max(0.0);
        } else {
            self.boundary_reservoir += -work;
        }
    }

    pub(crate) fn process_local(
        &mut self,
        controls: &LocalMediumBodyControls,
    ) -> Result<WorldBodyLocalFrame, ConstitutiveBodyError> {
        let energy_before = self.total_stored_energy();
        let requested_work = finite(controls.boundary_work_input).max(0.0) * self.dt;
        let boundary_input = requested_work
            .min((self.spec.boundary_reservoir_capacity - self.boundary_reservoir).max(0.0));
        self.boundary_reservoir += boundary_input;

        let port_input = self.port_input();
        let external_source_work = self.source_work_for_input(&controls.input)?;
        let mut combined = controls.input.clone();
        combined.drives.extend(port_input.drives);
        let combined_source_work = self.source_work_for_input(&combined)?;
        let port_source_work = combined_source_work - external_source_work;
        self.check_internal_transfer(port_source_work)?;
        let medium_frame = self.medium.step(&combined)?;
        self.settle_internal_transfer(port_source_work);
        let source_accounting_residual = medium_frame.source_work - combined_source_work;

        let output_scale = finite(controls.output_scale).clamp(0.0, 1.0);
        let boundary_before_output = kinetic(&self.boundary_momentum, self.dt);
        let mut next_boundary_momentum = self.boundary_momentum;
        if output_scale > 0.0 {
            let outputs = self
                .outputs
                .iter()
                .map(|binding| {
                    (
                        self.output_value(binding),
                        binding.boundary_direction,
                        binding.gain_per_second,
                    )
                })
                .collect::<Vec<_>>();
            for (value, direction, gain_per_second) in outputs {
                add_projection(
                    &mut next_boundary_momentum,
                    &direction,
                    output_scale * gain_per_second * value * self.dt,
                );
            }
        }
        let boundary_after_output = kinetic(&next_boundary_momentum, self.dt);
        let output_work = boundary_after_output - boundary_before_output;
        self.check_internal_transfer(output_work)?;
        self.boundary_momentum = next_boundary_momentum;
        self.settle_internal_transfer(output_work);

        let before_decay = kinetic(&self.boundary_momentum, self.dt);
        let decay = (-0.5 * self.spec.boundary_decay_per_second * self.dt).exp();
        for value in &mut self.boundary_momentum {
            *value *= decay;
        }
        let after_decay = kinetic(&self.boundary_momentum, self.dt);
        let boundary_loss = (before_decay - after_decay).max(0.0);

        let source_work = boundary_input + external_source_work;
        let internal_loss = medium_frame.internal_loss + boundary_loss;
        let energy_after = self.total_stored_energy();
        if !energy_after.is_finite()
            || self
                .boundary_momentum
                .iter()
                .any(|value| !value.is_finite())
        {
            return Err(ConstitutiveBodyError::NonFiniteState);
        }
        let closure_residual = energy_after - energy_before - source_work + internal_loss;
        let local_residual = medium_frame
            .closure_residual
            .abs()
            .max(medium_frame.transport_residual.abs())
            .max(medium_frame.positive_mass_residual.abs())
            .max(source_accounting_residual.abs());
        Ok(WorldBodyLocalFrame {
            source_work,
            internal_loss,
            closure_residual,
            local_residual,
        })
    }

    pub(crate) fn radiate(
        &mut self,
        fraction: f64,
    ) -> Result<WorldBodyRadiationFrame, ConstitutiveBodyError> {
        let fraction = finite(fraction).clamp(0.0, 1.0);
        let energy_before = kinetic(&self.boundary_momentum, self.dt);
        let projection = dot(&self.boundary_momentum, &self.radiation_direction);
        let retained_projection = (1.0 - fraction).sqrt() * projection;
        add_projection(
            &mut self.boundary_momentum,
            &self.radiation_direction,
            retained_projection - projection,
        );
        let emitted = fraction.sqrt() * projection;
        let radiation = 0.5 * self.dt * emitted * emitted;
        let energy_after = kinetic(&self.boundary_momentum, self.dt);
        Ok(WorldBodyRadiationFrame {
            emitted: vec![emitted],
            radiation,
            residual: energy_after - energy_before + radiation,
        })
    }
}

