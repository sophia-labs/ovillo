impl WorldBody {
    pub(crate) fn from_constitution(
        sample_rate_hz: f64,
        constitution: BodyConstitutionSpec,
    ) -> Result<Self, ConstitutiveBodyError> {
        match constitution {
            BodyConstitutionSpec::Modal { body } => {
                Ok(Self::Modal(Body::try_new(sample_rate_hz, body)?))
            }
            BodyConstitutionSpec::LocalMedium { boundary } => Ok(Self::LocalMedium(
                LocalMediumWorldBody::try_new(sample_rate_hz, boundary)?,
            )),
            BodyConstitutionSpec::ModalSpace { body } => {
                Ok(Self::ModalSpace(SpaceBody::try_new(sample_rate_hz, body)?))
            }
        }
    }

    #[must_use]
    pub(crate) fn total_stored_energy(&self) -> f64 {
        match self {
            Self::Modal(body) => body.total_stored_energy(),
            Self::LocalMedium(body) => body.total_stored_energy(),
            Self::ModalSpace(body) => body.total_stored_energy(),
        }
    }

    #[must_use]
    pub(crate) fn material_stored_energy(&self) -> f64 {
        match self {
            Self::Modal(body) => body.material_stored_energy(),
            Self::LocalMedium(body) => body.material_stored_energy(),
            Self::ModalSpace(body) => body.material_stored_energy(),
        }
    }

    #[must_use]
    pub(crate) fn reservoir_energy(&self) -> f64 {
        match self {
            Self::Modal(body) => body.reservoir_energy(),
            Self::LocalMedium(body) => body.reservoir_energy(),
            Self::ModalSpace(body) => body.reservoir_energy(),
        }
    }

    #[must_use]
    pub(crate) fn configuration_amount(&self) -> Option<f64> {
        match self {
            Self::Modal(body) => Some(body.configuration_amount()),
            Self::LocalMedium(_) | Self::ModalSpace(_) => None,
        }
    }

    #[must_use]
    pub(crate) fn field_state(&self) -> MetricFieldState<MODE_COUNT> {
        match self {
            Self::Modal(body) => body.field_state(),
            Self::LocalMedium(body) => MetricFieldState {
                displacement: [0.0; MODE_COUNT],
                momentum: *body.momentum(),
            },
            Self::ModalSpace(body) => body.field_state(),
        }
    }

    #[must_use]
    pub(crate) fn local_medium_state(&self) -> Option<&LocalMediumState> {
        match self {
            Self::Modal(_) | Self::ModalSpace(_) => None,
            Self::LocalMedium(body) => Some(body.state()),
        }
    }

    #[must_use]
    pub(crate) fn momentum(&self) -> &[f64; MODE_COUNT] {
        match self {
            Self::Modal(body) => body.momentum(),
            Self::LocalMedium(body) => body.momentum(),
            Self::ModalSpace(body) => body.momentum(),
        }
    }

    pub(crate) fn momentum_mut(&mut self) -> &mut [f64; MODE_COUNT] {
        match self {
            Self::Modal(body) => body.momentum_mut(),
            Self::LocalMedium(body) => body.momentum_mut(),
            Self::ModalSpace(body) => body.momentum_mut(),
        }
    }

    pub(crate) fn process_local(
        &mut self,
        controls: &ConstitutiveBodyControls,
        returning_space_activity: f64,
    ) -> Result<WorldBodyLocalFrame, ConstitutiveBodyError> {
        match (self, controls) {
            (Self::Modal(body), ConstitutiveBodyControls::Modal(controls)) => {
                let frame: BodyLocalFrame =
                    body.process_local(*controls, returning_space_activity)?;
                Ok(WorldBodyLocalFrame {
                    source_work: frame.source_work,
                    internal_loss: frame.internal_loss,
                    closure_residual: frame.closure_residual,
                    local_residual: frame.local_residual,
                })
            }
            (Self::LocalMedium(body), ConstitutiveBodyControls::LocalMedium(controls)) => {
                body.process_local(controls)
            }
            (Self::ModalSpace(body), ConstitutiveBodyControls::ModalSpace(controls)) => {
                let frame = body.process_local(controls, returning_space_activity)?;
                Ok(WorldBodyLocalFrame {
                    source_work: frame.source_work,
                    internal_loss: frame.internal_loss,
                    closure_residual: frame.closure_residual,
                    local_residual: frame.local_residual,
                })
            }
            _ => Err(ConstitutiveBodyError::ControlKindMismatch),
        }
    }

    pub(crate) fn radiate(
        &mut self,
        fraction: f64,
    ) -> Result<WorldBodyRadiationFrame, ConstitutiveBodyError> {
        match self {
            Self::Modal(body) => {
                let frame = body.radiate(fraction)?;
                Ok(WorldBodyRadiationFrame {
                    emitted: vec![frame.emitted_coordinate],
                    radiation: frame.radiation,
                    residual: frame.residual,
                })
            }
            Self::LocalMedium(body) => body.radiate(fraction),
            Self::ModalSpace(body) => {
                let mut emitted = Vec::with_capacity(4);
                let (radiation, residual) = body.radiate(fraction, &mut emitted)?;
                Ok(WorldBodyRadiationFrame {
                    emitted,
                    radiation,
                    residual,
                })
            }
        }
    }

    pub(crate) fn set_configuration_amount(
        &mut self,
        amount: f64,
    ) -> Result<BodyReconfigurationFrame, ConstitutiveBodyError> {
        match self {
            Self::Modal(body) => Ok(body.set_configuration_amount(amount)?),
            Self::LocalMedium(_) | Self::ModalSpace(_) => {
                Err(ConstitutiveBodyError::UnsupportedReconfiguration)
            }
        }
    }
}


impl WorldBody {
    pub(crate) fn space_body(&self) -> Option<&SpaceBody> {
        match self {
            Self::ModalSpace(body) => Some(body),
            _ => None,
        }
    }

    pub(crate) fn space_body_mut(&mut self) -> Option<&mut SpaceBody> {
        match self {
            Self::ModalSpace(body) => Some(body),
            _ => None,
        }
    }
}
