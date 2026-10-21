//! A persistent Ariadne body whose configuration is a point in an
//! N-dimensional space.
//!
//! Two constitutions share the 24-mode modal field, the reservoir and the
//! reconfiguration contract:
//!
//! - **modal** (multilinear map): the legacy body's laws, exactly as
//!   [`crate::body::Body`] runs them, configured from an N-D point;
//! - **tube network**: modes computed from the network at the point, a flow
//!   valve at one volume, wall loss and radiation diagonal in the modes.
//!
//! A move books its control work exactly; with the mass-momentum connection
//! it also books the energy the retained modes cannot hold (disposal).

use crate::configuration::network::{mass_momentum_transport, NetworkError, NetworkFrame, TubeNetwork};
use crate::configuration::space::MultilinearMap;
use crate::configuration::{
    ConfigurationMapSpec, ConnectionPolicy, DisposalPolicy, MAX_CONFIGURATION_DIMENSIONS,
};
use crate::body::BodyControls;
use crate::field::{MetricFieldState, ModalMetricWaveField};
use crate::ir::{MODE_COUNT, TRANSPORT_PAIRS};
use crate::law::active::{ActiveInclusionFrame, SynchronizedActiveInclusion};
use crate::law::aperture::CompliantApertureBoundary;
use crate::law::transport::InternalResonanceTransport;
use crate::law::aero::{AeroFrame, AeroRuntime, AeroTotals, Column};
use crate::law::valve::{FlowValve, FlowValveControls, FlowValveFrame};
use crate::plan::{SpaceBodyLaws, SpaceBodySpec};

/// Controls for an N-D body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpaceBodyControls {
    /// For both constitutions: `work_input` refills the reservoir. For the
    /// modal constitution every other field acts as on a legacy body.
    pub body: BodyControls,
    /// Tube network: the valve's supply (fraction of its maximum pressure).
    pub valve: FlowValveControls,
    /// Tube network: multiplier on the computed radiation (1 is the law,
    /// 0 removes radiation exactly).
    pub radiation_scale: f64,
    /// Tube network: an external volume velocity (m³/s) into one volume
    /// this sample (a knock on the wall); its exact work `pbar U dt` is
    /// booked as source work.
    pub node_source: Option<(usize, f64)>,
}

impl Default for SpaceBodyControls {
    fn default() -> Self {
        Self {
            body: BodyControls::default(),
            valve: FlowValveControls::default(),
            radiation_scale: 1.0,
            node_source: None,
        }
    }
}

/// Receipt of one move `x -> x'`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpaceReconfigurationFrame {
    pub dimensions: usize,
    pub point_before: [f64; MAX_CONFIGURATION_DIMENSIONS],
    pub point_after: [f64; MAX_CONFIGURATION_DIMENSIONS],
    pub energy_before: f64,
    pub energy_after: f64,
    /// Work the move did on the body (positive: the body gained).
    pub control_work: f64,
    /// Energy the retained modes could not hold (mass-momentum only).
    pub disposal: f64,
    pub disposal_policy: Option<DisposalPolicy>,
    /// Norm of the change of the modal state vector (zero for the
    /// modal-coordinate connection).
    pub state_displacement: f64,
    /// True when modes were recomputed from a tube network.
    pub modes_recomputed: bool,
}

/// The current tube-network modes, for analysis (non-loading).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NetworkModes {
    pub frequency_hz: [f64; MODE_COUNT],
    /// Wall-loss amplitude decay rate per mode (1/s).
    pub wall_sigma: [f64; MODE_COUNT],
    /// Radiation amplitude decay rate per mode and radiating end (1/s).
    pub radiation_sigma: [[f64; MODE_COUNT]; 4],
    /// Mode shapes at the valve's volume and at each radiating volume.
    pub valve_shape: [f64; MODE_COUNT],
    pub end_shapes: [[f64; MODE_COUNT]; 4],
    pub ends: usize,
    /// Share of each mode's potential energy that lies in tube 0.
    pub first_tube_fraction: [f64; MODE_COUNT],
}

/// What an observer may read from an N-D body (non-loading).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpaceBodyObservation {
    pub valve: FlowValveFrame,
    pub valve_half_gap_m: f64,
    pub valve_energy: f64,
    /// Pressure at each radiating end (up to 4), after the last radiation step.
    pub end_pressure_pa: [f64; 4],
    /// Radiation per end in the last step (J).
    pub end_radiation: [f64; 4],
    pub frequency_hz: [f64; MODE_COUNT],
    pub t60_seconds: [f64; MODE_COUNT],
    /// The air (POC-3), when the body declares it.
    pub aero: AeroFrame,
}

/// Cumulative energies of a tube-network body (J).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpaceTotals {
    pub reservoir_draw: f64,
    pub valve_field_work: f64,
    pub valve_jet_loss: f64,
    pub fold_loss: f64,
    pub wall_loss: f64,
    pub radiation_by_end: [f64; 4],
    /// `sum G_e p_e^2 dt`: what the radiating volumes' actual pressures
    /// radiate through their conductances. The booked radiation is the
    /// mode-diagonal law; the difference is that approximation's error
    /// (it drains static pressure that the physical law would not).
    pub physical_radiation_by_end: [f64; 4],
    /// Source work of external knocks (`node_source`).
    pub knock_work: f64,
    pub aero: AeroTotals,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpaceBodyError {
    InvalidSpec(String),
    Network(String),
    Field(String),
    Law(String),
    DisposalRejected,
    UnsupportedOperation(&'static str),
    NonFiniteState,
}

impl std::fmt::Display for SpaceBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSpec(m) => write!(f, "invalid N-D body: {m}"),
            Self::Network(m) => write!(f, "tube network: {m}"),
            Self::Field(m) => write!(f, "N-D body field: {m}"),
            Self::Law(m) => write!(f, "N-D body law: {m}"),
            Self::DisposalRejected => f.write_str("move refused: its disposal exceeds the reject tolerance"),
            Self::UnsupportedOperation(m) => write!(f, "unsupported on this constitution: {m}"),
            Self::NonFiniteState => f.write_str("N-D body entered a non-finite state"),
        }
    }
}

impl std::error::Error for SpaceBodyError {}

impl From<NetworkError> for SpaceBodyError {
    fn from(error: NetworkError) -> Self {
        Self::Network(format!("{error:?}"))
    }
}

/// Local frame reported to the World.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct SpaceLocalFrame {
    pub source_work: f64,
    pub internal_loss: f64,
    pub closure_residual: f64,
    pub local_residual: f64,
}

enum ModalMap {
    Multilinear(MultilinearMap),
    StiffString(crate::configuration::StiffStringSpec),
}

impl ModalMap {
    fn evaluate(&mut self, point: &[f64]) -> crate::ir::WorldConfiguration {
        match self {
            Self::Multilinear(map) => map.evaluate(point),
            Self::StiffString(string) => string.configuration_at(point),
        }
    }
}

struct ModalLaws {
    map: ModalMap,
    aperture: CompliantApertureBoundary<MODE_COUNT>,
    active: SynchronizedActiveInclusion<MODE_COUNT>,
    transport: InternalResonanceTransport<MODE_COUNT, TRANSPORT_PAIRS>,
    radiation_direction: [f64; MODE_COUNT],
}

struct NetworkLaws {
    network: TubeNetwork,
    frame: NetworkFrame,
    next: NetworkFrame,
    connection: ConnectionPolicy,
    valve: FlowValve,
    valve_node: usize,
    valve_shape: [f64; MODE_COUNT],
    end_shapes: Vec<[f64; MODE_COUNT]>,
    sigma_total: [f64; MODE_COUNT],
    excess: Vec<f64>,
    momentum: Vec<f64>,
    last_valve: FlowValveFrame,
    last_end_pressure: [f64; 4],
    last_end_radiation: [f64; 4],
    radiation_scale: f64,
    retain: [f64; MODE_COUNT],
    retain_scale: f64,
    removed: [f64; MODE_COUNT],
    aero: Option<AeroRuntime>,
    totals: SpaceTotals,
    dt: f64,
    /// DC-correct wall loss: per-mode retain factor on the column momenta.
    wall_retain_q: [f64; MODE_COUNT],
}

#[allow(clippy::large_enum_variant)]
enum Laws {
    Modal(ModalLaws),
    Network(NetworkLaws),
}

/// One persistent N-D body.
pub struct SpaceBody {
    spec: SpaceBodySpec,
    sample_rate_hz: f64,
    dt: f64,
    field: ModalMetricWaveField<MODE_COUNT>,
    point: [f64; MAX_CONFIGURATION_DIMENSIONS],
    dimensions: usize,
    reservoir_energy: f64,
    laws: Laws,
}

impl SpaceBody {
    pub fn try_new(sample_rate_hz: f64, spec: SpaceBodySpec) -> Result<Self, SpaceBodyError> {
        spec.validate(sample_rate_hz)
            .map_err(|error| SpaceBodyError::InvalidSpec(error.to_string()))?;
        let space = &spec.configuration_space;
        let dimensions = space.dimensions();
        let mut point = [0.0; MAX_CONFIGURATION_DIMENSIONS];
        point[..dimensions].copy_from_slice(&space.initial_point());
        let dt = sample_rate_hz.recip();
        let (field, laws) = match (&spec.laws, &space.map) {
            (SpaceBodyLaws::Modal { source_direction, active_directions, transport }, map_spec @ (ConfigurationMapSpec::Multilinear { .. } | ConfigurationMapSpec::StiffString { .. })) => {
                let mut map = match map_spec {
                    ConfigurationMapSpec::Multilinear { axes, anchors } => {
                        ModalMap::Multilinear(MultilinearMap::compile(axes, anchors))
                    }
                    ConfigurationMapSpec::StiffString { string } => ModalMap::StiffString(string.clone()),
                    ConfigurationMapSpec::TubeNetwork { .. } => unreachable!(),
                };
                let configuration = map.evaluate(&point[..dimensions]);
                let field = ModalMetricWaveField::try_new(sample_rate_hz, configuration.metric.runtime())
                    .map_err(|m| SpaceBodyError::Field(m.to_owned()))?;
                let aperture = CompliantApertureBoundary::try_new(
                    sample_rate_hz,
                    configuration.aperture.runtime(),
                    *source_direction,
                )
                .map_err(|e| SpaceBodyError::Law(e.to_string()))?;
                let active = SynchronizedActiveInclusion::try_new(sample_rate_hz, *active_directions)
                    .map_err(|e| SpaceBodyError::Law(e.to_string()))?;
                let transport = InternalResonanceTransport::try_new(sample_rate_hz, transport.pairs, transport.weights)
                    .map_err(|e| SpaceBodyError::Law(e.to_string()))?;
                (
                    field,
                    Laws::Modal(ModalLaws {
                        map,
                        aperture,
                        active,
                        transport,
                        radiation_direction: normalize(configuration.radiation_direction)?,
                    }),
                )
            }
            (SpaceBodyLaws::TubeNetwork { valve, aero }, ConfigurationMapSpec::TubeNetwork { network: network_spec }) => {
                let mut network = TubeNetwork::compile(network_spec, dimensions);
                let mut frame = network.empty_frame();
                let next = network.empty_frame();
                network.compute(&point[..dimensions], None, &mut frame)?;
                let field = ModalMetricWaveField::try_new(sample_rate_hz, frame.metric().runtime())
                    .map_err(|m| SpaceBodyError::Field(m.to_owned()))?;
                let valve_node = network.node(valve.tube, valve.section);
                let valve = FlowValve::try_new(sample_rate_hz, network_spec.air_density_kg_m3, valve.clone())
                    .map_err(|m| SpaceBodyError::Law(m.to_owned()))?;
                let ends = network.end_nodes().len();
                if ends > 4 {
                    return Err(SpaceBodyError::InvalidSpec("at most four radiating ends".into()));
                }
                let excess = vec![0.0; frame.nodes];
                let momentum = vec![0.0; frame.elements];
                let mut laws = NetworkLaws {
                    network,
                    frame,
                    next,
                    connection: space.connection,
                    valve,
                    valve_node,
                    valve_shape: [0.0; MODE_COUNT],
                    end_shapes: vec![[0.0; MODE_COUNT]; ends],
                    sigma_total: [0.0; MODE_COUNT],
                    excess,
                    momentum,
                    last_valve: FlowValveFrame::default(),
                    last_end_pressure: [0.0; 4],
                    last_end_radiation: [0.0; 4],
                    radiation_scale: 1.0,
                    retain: [1.0; MODE_COUNT],
                    retain_scale: f64::NAN,
                    removed: [0.0; MODE_COUNT],
                    aero: aero.filter(|spec| !spec.is_off()).map(|spec| {
                        AeroRuntime::new(spec, network_spec.air_density_kg_m3, network_spec.speed_of_sound_m_s)
                    }),
                    totals: SpaceTotals::default(),
                    dt,
                    wall_retain_q: [1.0; MODE_COUNT],
                };
                laws.refresh_shapes();
                (field, Laws::Network(laws))
            }
            _ => return Err(SpaceBodyError::InvalidSpec("laws and map do not match".into())),
        };
        Ok(Self {
            reservoir_energy: spec.reservoir.capacity,
            spec,
            sample_rate_hz,
            dt,
            field,
            point,
            dimensions,
            laws,
        })
    }

    #[must_use]
    pub fn spec(&self) -> &SpaceBodySpec {
        &self.spec
    }

    #[must_use]
    pub fn configuration_point(&self) -> &[f64] {
        &self.point[..self.dimensions]
    }

    #[must_use]
    pub fn field_state(&self) -> MetricFieldState<MODE_COUNT> {
        self.field.state()
    }

    #[must_use]
    pub fn frequencies_hz(&self) -> [f64; MODE_COUNT] {
        self.field.frequencies_hz()
    }

    #[must_use]
    pub fn material_stored_energy(&self) -> f64 {
        self.field.stored_energy()
            + match &self.laws {
                Laws::Modal(laws) => laws.aperture.stored_energy(),
                Laws::Network(laws) => laws.valve.stored_energy(),
            }
    }

    #[must_use]
    pub fn reservoir_energy(&self) -> f64 {
        self.reservoir_energy
    }

    #[must_use]
    pub fn total_stored_energy(&self) -> f64 {
        self.material_stored_energy() + self.reservoir_energy
    }

    #[must_use]
    pub fn field_energy(&self) -> f64 {
        self.field.stored_energy()
    }

    #[must_use]
    pub fn momentum(&self) -> &[f64; MODE_COUNT] {
        self.field.momentum()
    }

    pub(crate) fn momentum_mut(&mut self) -> &mut [f64; MODE_COUNT] {
        self.field.momentum_mut()
    }

    /// Non-loading observation.
    #[must_use]
    pub fn observation(&self) -> SpaceBodyObservation {
        let configuration = self.field.configuration();
        let mut observation = SpaceBodyObservation {
            frequency_hz: configuration.frequency_hz,
            t60_seconds: configuration.t60_seconds,
            ..Default::default()
        };
        if let Laws::Network(laws) = &self.laws {
            observation.valve = laws.last_valve;
            observation.valve_half_gap_m = laws.valve.half_gap_m();
            observation.valve_energy = laws.valve.stored_energy();
            observation.end_pressure_pa = laws.last_end_pressure;
            observation.end_radiation = laws.last_end_radiation;
            if let Some(aero) = &laws.aero {
                observation.aero = aero.last;
            }
        }
        observation
    }

    /// The valve's half-gap (m) and the jet's last Reynolds number, cheaply
    /// (non-loading; zeros for a modal constitution).
    #[must_use]
    pub fn valve_gap_and_reynolds(&self) -> (f64, f64) {
        match &self.laws {
            Laws::Network(laws) => (laws.valve.half_gap_m(), laws.aero.as_ref().map_or(0.0, |a| a.last.reynolds)),
            Laws::Modal(_) => (0.0, 0.0),
        }
    }

    /// Cumulative energies (`None` for a modal constitution).
    #[must_use]
    pub fn totals(&self) -> Option<SpaceTotals> {
        match &self.laws {
            Laws::Network(laws) => {
                let mut totals = laws.totals;
                if let Some(aero) = &laws.aero {
                    totals.aero = aero.totals;
                }
                Some(totals)
            }
            Laws::Modal(_) => None,
        }
    }

    /// Mode shapes `phi_k(node)` at a network volume (non-loading).
    #[must_use]
    pub fn node_shape(&self, node: usize) -> Option<[f64; MODE_COUNT]> {
        let Laws::Network(laws) = &self.laws else { return None };
        (node < laws.frame.nodes).then(|| laws.frame.shape_at(node))
    }

    /// Pressure (Pa) at a network volume, from the retained modes
    /// (non-loading).
    #[must_use]
    pub fn node_pressure(&self, node: usize) -> Option<f64> {
        let Laws::Network(laws) = &self.laws else { return None };
        let frame = &laws.frame;
        if node >= frame.nodes {
            return None;
        }
        let momentum = self.field.momentum();
        Some(self.dt.sqrt() * (0..MODE_COUNT).map(|k| frame.phi[k * frame.nodes + node] * momentum[k]).sum::<f64>())
    }

    /// The current tube-network modes (`None` for a modal constitution).
    #[must_use]
    pub fn network_modes(&self) -> Option<NetworkModes> {
        let Laws::Network(laws) = &self.laws else { return None };
        let frame = &laws.frame;
        let mut modes = NetworkModes {
            frequency_hz: frame.frequency_hz,
            valve_shape: laws.valve_shape,
            ends: laws.end_shapes.len(),
            ..Default::default()
        };
        let first_tube_nodes = self.spec_first_tube_nodes();
        for k in 0..MODE_COUNT {
            modes.wall_sigma[k] = if frame.t60_seconds[k] < f64::MAX { 6.907_755_278_982_137 / frame.t60_seconds[k] } else { 0.0 };
            let mut first = 0.0;
            let mut total = 0.0;
            for node in 0..frame.nodes {
                let w = frame.compliance[node] * frame.phi[k * frame.nodes + node].powi(2);
                total += w;
                if node < first_tube_nodes {
                    first += w;
                }
            }
            modes.first_tube_fraction[k] = first / total;
        }
        for e in 0..modes.ends {
            modes.radiation_sigma[e] = frame.radiation_sigma[e];
            modes.end_shapes[e] = laws.end_shapes[e];
        }
        Some(modes)
    }

    fn spec_first_tube_nodes(&self) -> usize {
        match &self.spec.configuration_space.map {
            ConfigurationMapSpec::TubeNetwork { network } => network.tubes[0].base_log_area_m2.len(),
            _ => 0,
        }
    }

    /// Areas (m²) of the current tube-network configuration.
    #[must_use]
    pub fn network_areas_m2(&self) -> Option<&[f64]> {
        match &self.laws {
            Laws::Network(laws) => Some(&laws.frame.areas_m2),
            Laws::Modal(_) => None,
        }
    }

    /// Move to `point` (clamped into the box) at fixed physical state, under
    /// the space's declared connection.
    pub fn set_configuration_point(
        &mut self,
        point: &[f64],
    ) -> Result<SpaceReconfigurationFrame, SpaceBodyError> {
        let dims = self.dimensions;
        let mut target = [0.0; MAX_CONFIGURATION_DIMENSIONS];
        self.spec.configuration_space.clamp_into(point, &mut target[..dims]);
        let mut frame = SpaceReconfigurationFrame {
            dimensions: dims,
            point_before: self.point,
            point_after: target,
            ..Default::default()
        };
        if (0..dims).all(|i| target[i].to_bits() == self.point[i].to_bits()) {
            let energy = self.field.stored_energy();
            frame.energy_before = energy;
            frame.energy_after = energy;
            return Ok(frame);
        }
        match &mut self.laws {
            Laws::Modal(laws) => {
                let configuration = laws.map.evaluate(&target[..dims]);
                configuration
                    .validate(self.sample_rate_hz)
                    .map_err(|error| SpaceBodyError::InvalidSpec(error.to_string()))?;
                let energy_before = self.field.stored_energy() + laws.aperture.stored_energy();
                let field = self
                    .field
                    .set_configuration(configuration.metric.runtime())
                    .map_err(|m| SpaceBodyError::Field(m.to_owned()))?;
                let aperture = laws
                    .aperture
                    .set_configuration(configuration.aperture.runtime())
                    .map_err(|e| SpaceBodyError::Law(e.to_string()))?;
                laws.radiation_direction = normalize(configuration.radiation_direction)?;
                frame.energy_before = energy_before;
                frame.energy_after = self.field.stored_energy() + laws.aperture.stored_energy();
                frame.control_work = field.control_work + aperture.control_work;
                frame.state_displacement = field.state_displacement + aperture.state_displacement;
            }
            Laws::Network(laws) => {
                let NetworkLaws { network, frame: current, next, .. } = laws;
                network.compute(&target[..dims], Some(current), next)?;
                let energy_before = self.field.stored_energy();
                let state_before = self.field.state();
                frame.energy_before = energy_before;
                frame.modes_recomputed = true;
                match laws.connection {
                    ConnectionPolicy::ModalCoordinates => {
                        let work = self
                            .field
                            .set_configuration(laws.next.metric().runtime())
                            .map_err(|m| SpaceBodyError::Field(m.to_owned()))?;
                        frame.control_work = work.control_work;
                        frame.energy_after = self.field.stored_energy();
                    }
                    ConnectionPolicy::MassMomentum { disposal } => {
                        let mut state = state_before;
                        let moved = mass_momentum_transport(
                            &laws.frame,
                            &laws.next,
                            self.dt,
                            &mut state,
                            &mut laws.excess,
                            &mut laws.momentum,
                        );
                        let previous_configuration = self.field.configuration();
                        self.field
                            .set_configuration(laws.next.metric().runtime())
                            .map_err(|m| SpaceBodyError::Field(m.to_owned()))?;
                        self.field
                            .restore_state(state)
                            .map_err(|m| SpaceBodyError::Field(m.to_owned()))?;
                        let energy_after = self.field.stored_energy();
                        let truncated = (moved.physical_energy_after - energy_after).max(0.0);
                        if disposal == DisposalPolicy::Reject
                            && truncated > 1.0e-9 * energy_before.max(moved.physical_energy_after) + 1.0e-300
                        {
                            self.field
                                .set_configuration(previous_configuration)
                                .map_err(|m| SpaceBodyError::Field(m.to_owned()))?;
                            self.field
                                .restore_state(state_before)
                                .map_err(|m| SpaceBodyError::Field(m.to_owned()))?;
                            return Err(SpaceBodyError::DisposalRejected);
                        }
                        frame.energy_after = energy_after;
                        frame.disposal = truncated;
                        frame.disposal_policy = Some(disposal);
                        // Closure by construction: after = before + work - disposal.
                        frame.control_work = energy_after - energy_before + truncated;
                        let after = self.field.state();
                        frame.state_displacement = (0..MODE_COUNT)
                            .map(|k| {
                                (after.momentum[k] - state_before.momentum[k]).powi(2)
                                    + (after.displacement[k] - state_before.displacement[k]).powi(2)
                            })
                            .sum::<f64>()
                            .sqrt();
                    }
                }
                std::mem::swap(&mut laws.frame, &mut laws.next);
                laws.refresh_shapes();
            }
        }
        self.point = target;
        Ok(frame)
    }

    /// Retune the valve (tension, adduction) at fixed fold state; returns the
    /// control work.
    pub fn retune_valve(&mut self, tuning_hz: f64, adduction: f64) -> Result<f64, SpaceBodyError> {
        match &mut self.laws {
            Laws::Network(laws) => Ok(laws.valve.retune(tuning_hz, adduction)),
            Laws::Modal(_) => Err(SpaceBodyError::UnsupportedOperation("this body has no flow valve")),
        }
    }

    /// Advance every body-local law except shared-space exchange, direct
    /// coupling, and radiation (the World's order, as for a legacy body).
    pub(crate) fn process_local(
        &mut self,
        controls: &SpaceBodyControls,
        returning_space_activity: f64,
    ) -> Result<SpaceLocalFrame, SpaceBodyError> {
        // The public engine's lean path (DC-correct air): per-mode loss
        // booking, no diagnostic sums; the World's ledger still measures
        // every frame's closure.
        if let Laws::Network(laws) = &self.laws {
            if laws.aero.as_ref().is_some_and(|a| a.spec.dc_wall_loss) {
                return self.process_local_lean(controls);
            }
        }
        let body_controls = controls.body.sanitized();
        let energy_before = self.total_stored_energy();
        let capacity = self.spec.reservoir.capacity;
        let gap = (capacity - self.reservoir_energy).max(0.0);
        let work_input = (body_controls.work_input * self.dt).min(gap);
        self.reservoir_energy += work_input;
        let (source_work, internal_loss, local_residual) = match &mut self.laws {
            Laws::Modal(laws) => {
                // Exactly `Body::process_local` after the reservoir refill.
                let aperture_direction = *laws.aperture.direction();
                let field_effort = self
                    .field
                    .aperture_effort(&aperture_direction)
                    .map_err(|m| SpaceBodyError::Field(m.to_owned()))?;
                let aperture = laws.aperture.process(self.field.momentum_mut(), field_effort, body_controls.source_wave);
                if aperture.non_finite_input {
                    return Err(SpaceBodyError::NonFiniteState);
                }
                let boundary_activity = body_controls.source_wave.abs()
                    + aperture.field_flow_after.abs()
                    + 0.5 * laws.aperture.state().momentum.abs();
                let active = if body_controls.active_coupling > 0.0 {
                    let mut active_controls = body_controls.active;
                    active_controls.drive *= body_controls.active_coupling;
                    laws.active.process(
                        self.field.momentum_mut(),
                        body_controls.active_coupling * boundary_activity,
                        finite(returning_space_activity).abs(),
                        active_controls,
                        &mut self.reservoir_energy,
                    )
                } else {
                    ActiveInclusionFrame::default()
                };
                if active.non_finite_state {
                    return Err(SpaceBodyError::NonFiniteState);
                }
                let displacement = *self.field.displacement();
                let transport = laws.transport.process(&displacement, self.field.momentum_mut(), body_controls.transport);
                if transport.non_finite_state {
                    return Err(SpaceBodyError::NonFiniteState);
                }
                let propagation = self.field.propagate();
                let field_loss = self.field.damp();
                let source = work_input + aperture.source_boundary_work + aperture.active_source_work;
                let loss = aperture.internal_loss + active.dissipated_work + field_loss.loss;
                let residual = [
                    aperture.closure_residual,
                    active.closure_residual,
                    transport.metric_residual,
                    propagation.residual,
                    field_loss.residual,
                ]
                .into_iter()
                .map(f64::abs)
                .fold(0.0, f64::max);
                (source, loss, residual)
            }
            Laws::Network(laws) => {
                laws.radiation_scale = if controls.radiation_scale.is_finite() {
                    controls.radiation_scale.max(0.0)
                } else {
                    0.0
                };
                let valve = laws.valve.process(
                    self.field.momentum_mut(),
                    &laws.valve_shape,
                    controls.valve,
                    &mut self.reservoir_energy,
                    capacity,
                );
                laws.last_valve = valve;
                let mut knock = 0.0;
                if let Some((node, flow)) = controls.node_source {
                    if node < laws.frame.nodes && flow.is_finite() && flow != 0.0 {
                        let shape = laws.frame.shape_at(node);
                        let sqrt_dt = self.dt.sqrt();
                        let momentum = self.field.momentum_mut();
                        let mut norm2 = 0.0;
                        let mut projection = 0.0;
                        for k in 0..MODE_COUNT {
                            norm2 += shape[k] * shape[k];
                            projection += shape[k] * momentum[k];
                        }
                        let p_bar = sqrt_dt * projection + 0.5 * self.dt * norm2 * flow;
                        for k in 0..MODE_COUNT {
                            momentum[k] += sqrt_dt * shape[k] * flow;
                        }
                        knock = p_bar * flow * self.dt;
                        laws.totals.knock_work += knock;
                    }
                }
                let mut aero_loss = 0.0;
                let mut dc_wall_loss = false;
                if let Some(aero) = &mut laws.aero {
                    let frame = aero.step(self.field.displacement_mut(), self.dt);
                    // The field changed by turbulence_work - jet_loss.
                    aero_loss = frame.jet_loss - frame.turbulence_work;
                    dc_wall_loss = aero.spec.dc_wall_loss;
                }
                let propagation = self.field.propagate();
                let field_loss = if dc_wall_loss {
                    self.field.damp_displacement(&laws.wall_retain_q)
                } else {
                    self.field.damp()
                };
                let loss = valve.jet_loss + valve.fold_loss + field_loss.loss + aero_loss;
                let t = &mut laws.totals;
                t.reservoir_draw += valve.reservoir_draw;
                t.valve_field_work += valve.field_work;
                t.valve_jet_loss += valve.jet_loss;
                t.fold_loss += valve.fold_loss;
                t.wall_loss += field_loss.loss;
                let residual = propagation.residual.abs().max(field_loss.residual.abs());
                (work_input + knock, loss, residual)
            }
        };
        let energy_after = self.total_stored_energy();
        if !energy_after.is_finite() || !self.reservoir_energy.is_finite() || self.reservoir_energy < -1.0e-15 {
            return Err(SpaceBodyError::NonFiniteState);
        }
        Ok(SpaceLocalFrame {
            source_work,
            internal_loss,
            closure_residual: energy_after - energy_before - source_work + internal_loss,
            local_residual,
        })
    }

    fn process_local_lean(&mut self, controls: &SpaceBodyControls) -> Result<SpaceLocalFrame, SpaceBodyError> {
        let capacity = self.spec.reservoir.capacity;
        let refill = if controls.body.work_input.is_finite() { controls.body.work_input.clamp(0.0, 1.0) } else { 0.0 };
        let gap = (capacity - self.reservoir_energy).max(0.0);
        let work_input = (refill * self.dt).min(gap);
        self.reservoir_energy += work_input;
        let Laws::Network(laws) = &mut self.laws else { unreachable!() };
        laws.radiation_scale = if controls.radiation_scale.is_finite() { controls.radiation_scale.max(0.0) } else { 0.0 };
        let valve = laws.valve.process(self.field.momentum_mut(), &laws.valve_shape, controls.valve, &mut self.reservoir_energy, capacity);
        laws.last_valve = valve;
        let mut knock = 0.0;
        if let Some((node, flow)) = controls.node_source {
            if node < laws.frame.nodes && flow.is_finite() && flow != 0.0 {
                let shape = laws.frame.shape_at(node);
                let sqrt_dt = self.dt.sqrt();
                let momentum = self.field.momentum_mut();
                let mut norm2 = 0.0;
                let mut projection = 0.0;
                for k in 0..MODE_COUNT {
                    norm2 += shape[k] * shape[k];
                    projection += shape[k] * momentum[k];
                }
                let p_bar = sqrt_dt * projection + 0.5 * self.dt * norm2 * flow;
                for k in 0..MODE_COUNT {
                    momentum[k] += sqrt_dt * shape[k] * flow;
                }
                knock = p_bar * flow * self.dt;
                laws.totals.knock_work += knock;
            }
        }
        let aero = laws.aero.as_mut().expect("lean path has air").step(self.field.displacement_mut(), self.dt);
        self.field.propagate_lean();
        let wall = self.field.damp_displacement_lean(&laws.wall_retain_q);
        let t = &mut laws.totals;
        t.reservoir_draw += valve.reservoir_draw;
        t.valve_field_work += valve.field_work;
        t.valve_jet_loss += valve.jet_loss;
        t.fold_loss += valve.fold_loss;
        t.wall_loss += wall;
        if !self.reservoir_energy.is_finite() || self.reservoir_energy < -1.0e-15 || self.field.momentum().iter().any(|v| !v.is_finite()) {
            return Err(SpaceBodyError::NonFiniteState);
        }
        Ok(SpaceLocalFrame {
            source_work: work_input + knock,
            internal_loss: valve.jet_loss + valve.fold_loss + wall + aero.jet_loss - aero.turbulence_work,
            closure_residual: 0.0,
            local_residual: 0.0,
        })
    }

    /// Radiate. Modal constitution: remove `fraction` of the projection on the
    /// configured direction (the legacy law). Tube network: the computed
    /// per-mode radiation, scaled by the last controls' `radiation_scale`;
    /// `fraction` is ignored. Returns `(emitted, radiation, residual)`.
    pub(crate) fn radiate(&mut self, fraction: f64, emitted: &mut Vec<f64>) -> Result<(f64, f64), SpaceBodyError> {
        match &mut self.laws {
            Laws::Modal(laws) => {
                let frame = self
                    .field
                    .radiate(&laws.radiation_direction, finite(fraction).clamp(0.0, 1.0))
                    .map_err(|m| SpaceBodyError::Field(m.to_owned()))?;
                emitted.clear();
                emitted.push(frame.emitted_coordinate);
                Ok((frame.radiation, frame.residual))
            }
            Laws::Network(laws) => {
                // DC-correct air: radiation through the radiating volumes'
                // conductance acts on pressure (the momentum coordinates) only,
                // at twice the rate (same decay per mode; a steady flow out of
                // the mouth radiates nothing).
                let dc = laws.aero.as_ref().is_some_and(|a| a.spec.dc_wall_loss);
                if laws.retain_scale.to_bits() != laws.radiation_scale.to_bits() {
                    let factor = if dc { 2.0 } else { 1.0 };
                    for k in 0..MODE_COUNT {
                        laws.retain[k] = (-factor * laws.radiation_scale * laws.sigma_total[k] * self.dt).exp();
                    }
                    laws.retain_scale = laws.radiation_scale;
                }
                let loss = if dc {
                    let loss = self.field.damp_momentum_lean(&laws.retain, &mut laws.removed);
                    crate::field::MetricLossFrame { loss, ..Default::default() }
                } else {
                    self.field.damp_modes(&laws.retain, &mut laws.removed)
                };
                let ends = laws.end_shapes.len();
                let sqrt_dt = self.dt.sqrt();
                let momentum = self.field.momentum();
                emitted.clear();
                for e in 0..ends {
                    let shape = &laws.end_shapes[e];
                    let pressure = sqrt_dt * (0..MODE_COUNT).map(|k| shape[k] * momentum[k]).sum::<f64>();
                    laws.last_end_pressure[e] = pressure;
                    emitted.push(pressure);
                    let mut radiated = 0.0;
                    for k in 0..MODE_COUNT {
                        let total = laws.sigma_total[k];
                        if total > 0.0 {
                            radiated += laws.removed[k] * laws.frame.radiation_sigma[e][k] / total;
                        }
                    }
                    laws.last_end_radiation[e] = radiated;
                    laws.totals.radiation_by_end[e] += radiated;
                    laws.totals.physical_radiation_by_end[e] += laws.frame.radiation_conductance[e] * pressure * pressure * self.dt;
                }
                Ok((loss.loss, loss.residual))
            }
        }
    }

    pub fn reset(&mut self) -> Result<(), SpaceBodyError> {
        *self = Self::try_new(self.sample_rate_hz, self.spec.clone())?;
        Ok(())
    }
}

impl NetworkLaws {
    fn refresh_shapes(&mut self) {
        self.valve_shape = self.frame.shape_at(self.valve_node);
        for (e, &node) in self.network.end_nodes().iter().enumerate() {
            self.end_shapes[e] = self.frame.shape_at(node);
        }
        for k in 0..MODE_COUNT {
            self.sigma_total[k] = self.frame.radiation_sigma.iter().map(|s| s[k]).sum();
        }
        self.retain_scale = f64::NAN;
        self.refresh_air();
    }

    /// The air's columns at this configuration, from the network's contact:
    /// the jet column and its obstacle downstream; the DC-correct wall loss.
    fn refresh_air(&mut self) {
        let Some(aero) = &mut self.aero else { return };
        let frame = &self.frame;
        aero.jet = None;
        aero.obstacle = None;
        aero.port_closed = frame.port.is_some_and(|(_, open)| open <= 0.0);
        if let Some((junction, element, open)) = frame.jet {
            let column = |e: usize| Column::new(e, &frame.flux, frame.elements, frame.inertance[e], &frame.omega2);
            aero.jet_junction = junction;
            aero.jet_aperture = open;
            aero.jet = Some(column(element));
            let tube = self.network.contact_tube().unwrap_or(0);
            let last = self.network.tube_sections(tube) - 2;
            let obstacle = (junction + aero.spec.obstacle_offset).min(last);
            aero.obstacle = Some(column(self.network.junction_element(tube, obstacle)));
        }
        if aero.spec.dc_wall_loss {
            for k in 0..MODE_COUNT {
                let sigma = if frame.t60_seconds[k] < f64::MAX { 6.907_755_278_982_137 / frame.t60_seconds[k] } else { 0.0 };
                self.wall_retain_q[k] = (-2.0 * sigma * self.dt).exp();
            }
        }
    }
}

/// The legacy `Body` normalizes the (already normalized) interpolated
/// direction once more; do the same so a 1-D space is bitwise a legacy body.
fn normalize(mut direction: [f64; MODE_COUNT]) -> Result<[f64; MODE_COUNT], SpaceBodyError> {
    let norm = direction.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 {
        return Err(SpaceBodyError::Law("radiation direction must be finite and nondegenerate".into()));
    }
    for value in &mut direction {
        *value /= norm;
    }
    Ok(direction)
}

#[inline]
fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}
