// Types for the portable Ovillo package (ovillo.js, core.js). The catalog
// (ovillo-catalog.json) is the source of truth for kinds, events, controls
// and their units; these types describe its shape and the API around it.

export interface ArgDef {
  name: string;
  units: string;
  min: number;
  max: number;
  default: number;
  /** Named values: pass the name (or its index). */
  values?: string[];
  about: string;
}

export interface EventDef {
  name: string;
  about: string;
  args: ArgDef[];
}

export interface ControlDef {
  name: string;
  units: string;
  min: number;
  max: number;
  default: number;
  /** A sensible update rate from a game loop. */
  rateHz: number;
  /** What the World does between updates. */
  smoothing: string;
  /** Discrete (a switch or an index): never ramped. */
  steps: boolean;
  values?: string[];
  about: string;
}

export interface KindDef {
  id: string;
  title: string;
  family: "tab" | "piano" | "trombone" | "tract" | "fart" | "bench" | string;
  summary: string;
  spec: string;
  /** The only sample rate this World runs at (its plan's), or null for any. */
  sampleRate: number | null;
  /** 1: heard at one point (left = right); 2: two listening points. */
  channels: 1 | 2;
  anatomies: { id: string; title: string; about: string; default: boolean }[];
  options: { name: string; type: string; about: string; default: unknown }[];
  events: EventDef[];
  controls: ControlDef[];
  telemetry: { name: string; units: string; about: string }[];
  score: { format: string; about: string } | null;
  output: { observes: string; gain: number; channels: string; about: string };
  lookaheadSeconds: number;
  cost: string;
  doesNotModel: string[];
  doesNotModelRef: string;
}

export interface Catalog {
  schema: "ovillo.portable.catalog.v1";
  producer: string;
  about: string;
  commonTelemetry: string[];
  kinds: KindDef[];
}

export interface Ledger {
  sourceWork: number;
  controlWork: number;
  internalLoss: number;
  radiation: number;
  initialEnergy: number;
  currentEnergy: number;
  cumulativeClosureResidual: number;
  maximumFrameResidual: number;
  globalClosureResidual: number;
  sampleFrames: number;
  closureTolerance: number;
  closes: boolean;
}

export interface Receipt {
  schema: "ovillo.portable.receipt.v1";
  producer: string;
  kind: string;
  family: string;
  anatomy: string;
  options: Record<string, unknown>;
  sampleRate: number;
  frames: number;
  seconds: number;
  planDigest: string;
  /** sha256 of every frame rendered, interleaved left/right, f32 little-endian. */
  outputDigest: string;
  outputDigestCovers: string;
  output: KindDef["output"];
  events: { applied: number; refused: number; lastError: string; errors: number };
  clamped: number;
  scores: unknown[];
  ledger: Ledger;
  /** The World's receipt in its own renderer's shape. */
  world: Record<string, unknown>;
  doesNotModel: { see: string; items: string[] };
  claimBoundary: string;
}

/** Common slots first (Catalog.commonTelemetry), then the kind's. */
export type Telemetry = Record<string, number>;

export type EventArgs = Record<string, number | string | boolean>;

export interface CreateOptions {
  anatomy?: string;
  /** Kind-specific (catalog `options`): tuning, hands, keys, schedule, ... */
  options?: Record<string, unknown>;
  /** A score in the kind's format (catalog `score`), played from the start. */
  score?: string;
  /** 1 makes a point source (two listening points downmixed); default the kind's own. */
  channels?: 1 | 2;
  telemetryHz?: number;
}

export interface At {
  /** AudioContext time in seconds; omitted: the next render quantum. */
  at?: number;
}

export interface SetOptions extends At {
  /** Seconds of linear travel to the new value. */
  ramp?: number;
}

export declare class World {
  /** A real AudioNode: connect it to a PannerNode, a gain, the destination. */
  readonly node: AudioWorkletNode;
  readonly kind: string;
  readonly def: KindDef;
  readonly channels: 1 | 2;
  info: { kind: string; anatomy: string; sampleRate: number; planDigest: string; channels: number; lookaheadFrames: number; telemetry: string[] } | null;
  /** Updated about 30 times a second. */
  telemetry: Telemetry;
  lastError: string | null;
  onTelemetry: ((t: Telemetry) => void) | null;
  onWarning: (message: string) => void;
  readonly ready: Promise<World>;
  set(name: string, value: number | string | boolean, opts?: SetOptions): void;
  set(values: Record<string, number | string | boolean>, opts?: SetOptions): void;
  event(name: string, args?: EventArgs, opts?: At): void;
  play(score: string, opts?: At): Promise<Record<string, unknown>>;
  receipt(): Promise<Receipt>;
  reset(): Promise<void>;
  dispose(): void;
}

export interface RenderSpec {
  kind: string;
  anatomy?: string;
  options?: Record<string, unknown>;
  /** Alias of options. */
  plan?: Record<string, unknown>;
  score?: string;
  seconds?: number;
  frames?: number;
  /** After a score's last note, when seconds is not given. */
  tailSeconds?: number;
  sampleRate?: number;
  events?: { at?: number; name: string; args?: EventArgs }[];
  controls?: { at?: number; name: string; value: number | string | boolean; ramp?: number }[];
  block?: number;
  /** Where the module comes from (default: ovillo.wasm beside ovillo.js). */
  wasm?: string | URL | ArrayBuffer | Uint8Array;
  module?: WebAssembly.Module;
}

export interface Rendered {
  left: Float32Array;
  right: Float32Array;
  sampleRate: number;
  frames: number;
  channels: 1 | 2;
  receipt: Receipt;
  wallSeconds: number;
}

export declare class Ovillo {
  static load(context: BaseAudioContext, opts?: { wasmUrl?: string | URL; workletUrl?: string | URL; module?: WebAssembly.Module }): Promise<Ovillo>;
  static render(spec: RenderSpec): Promise<Rendered>;
  static catalog(src?: string | URL | ArrayBuffer | Uint8Array | WebAssembly.Module): Promise<Catalog>;
  readonly context: BaseAudioContext;
  readonly module: WebAssembly.Module;
  readonly catalog: Catalog;
  kinds(): string[];
  kind(id: string): KindDef;
  create(kind: string, opts?: CreateOptions): Promise<World>;
  render(spec: RenderSpec): Rendered;
}

export declare class OvilloCore {
  static instantiate(module: WebAssembly.Module): OvilloCore;
  static load(src: ArrayBuffer | Uint8Array | WebAssembly.Module): Promise<OvilloCore>;
  readonly maxFrames: number;
  catalog(): Catalog;
  kind(id: string): KindDef;
  create(kind: string, spec?: { anatomy?: string; options?: Record<string, unknown> }, sampleRate?: number): CoreWorld;
  instances(): number;
}

export declare class CoreWorld {
  readonly kind: string;
  readonly def: KindDef;
  readonly slotNames: string[];
  /** frame: the World frame the event lands on (-1 or omitted: now). */
  event(name: string, args?: EventArgs, frame?: number): void;
  set(name: string, value: number | string | boolean, frame?: number, rampFrames?: number): void;
  control(name: string): number;
  playScore(text: string, frame?: number): Record<string, unknown>;
  render(frames: number): number;
  left(n: number): Float32Array;
  right(n: number): Float32Array;
  telemetry(): Telemetry;
  telemetryArray(): Float64Array;
  frame(): number;
  refresh(): void;
  receipt(): Receipt;
  lastError(): string;
  reset(): void;
  dispose(): void;
}

export declare function renderOffline(core: OvilloCore, spec: RenderSpec): Rendered;

export default Ovillo;
