// What each World kind plays for its wasm receipt and its real-time factor:
// the same touches as ovillo-portable's tests/portable.rs, as RenderSpecs.
import { readFileSync } from "node:fs";

const root = new URL("../../../", import.meta.url);
const read = (p) => readFileSync(new URL(p, root), "utf8");

export function scenario(kind, catalog) {
  const def = catalog.kinds.find((k) => k.id === kind);
  switch (kind) {
    case "steel-string":
      return { kind, score: read("examples/tab/steel_string_tour.mus"), seconds: 6 };
    case "piano":
      return {
        kind,
        options: { keys: ["C4", "E4", "G4", "C5"] },
        seconds: 4,
        events: [
          { at: 0.0, name: "note", args: { pitch: 60, velocity: 0.6, duration: 0.8 } },
          { at: 0.5, name: "note", args: { pitch: 64, velocity: 0.5, duration: 0.8 } },
          { at: 1.0, name: "key", args: { pitch: 67, velocity: 0.7 } },
          { at: 1.5, name: "key", args: { pitch: 72, velocity: 0.8 } },
          { at: 2.0, name: "release", args: { pitch: 67 } },
          { at: 2.0, name: "release", args: { pitch: 72 } },
        ],
        controls: [{ at: 0.9, name: "pedal", value: "down" }, { at: 2.8, name: "pedal", value: "up" }],
      };
    case "trombone":
      return { kind, score: read("examples/ensemble/before-the-law/doorkeeper/doorkeeper-audition.tsv"), seconds: 8 };
    case "tract":
      return {
        kind,
        seconds: 4,
        controls: [{ at: 0, name: "breath", value: 0.6, ramp: 0.05 }, { at: 3.5, name: "breath", value: 0, ramp: 0.1 }],
        events: [
          { at: 0, name: "preset", args: { vowel: "a" } },
          { at: 1, name: "preset", args: { vowel: "i" } },
          { at: 1.6, name: "gesture", args: { name: "sss" } },
          { at: 2.6, name: "constrict", args: { slot: 1, position: 0.86, degree: 0.93 } },
          { at: 3.0, name: "unconstrict", args: { slot: 1 } },
          { at: 3.2, name: "impact", args: { position: 0.5, energy: 1e-4 } },
        ],
      };
    case "fart":
      return {
        kind,
        seconds: 5,
        events: [
          { at: 0, name: "eat", args: { food: "beans" } },
          { at: 0, name: "eat", args: { food: "soda" } },
          { at: 2, name: "style", args: { name: "brap" } },
        ],
      };
    case "bench":
      return { kind, options: { seed: 7 }, seconds: 4, events: [{ at: 0, name: "pattern", args: { name: "wander" } }, { at: 0, name: "transport", args: { on: 1 } }] };
    case "kart_engine":
      return {
        kind,
        options: { seed: 3 },
        seconds: 6,
        events: [{ at: 0, name: "start" }, { at: 3.2, name: "bump", args: { impulse: 0.7 } }, { at: 3.5, name: "land", args: { impulse: 0.5 } }, { at: 4, name: "hit" }, { at: 5, name: "restart" }],
        controls: [
          { at: 0, name: "throttle", value: 0.6 },
          { at: 0, name: "rpm", value: 3000 },
          { at: 0.8, name: "rpm", value: 9000, ramp: 1.5 },
          { at: 2.5, name: "boost", value: 0.8, ramp: 0.2 },
          { at: 3, name: "skid", value: 0.5, ramp: 0.1 },
          { at: 3, name: "surface", value: "lakebed" },
          { at: 3.8, name: "load", value: 0, ramp: 0.2 },
        ],
      };
    case "race_props":
      return {
        kind,
        seconds: 9,
        events: [
          { at: 0, name: "beep", args: { n: 3 } }, { at: 1, name: "beep", args: { n: 2 } }, { at: 2, name: "beep", args: { n: 1 } },
          { at: 3, name: "go" }, { at: 4.5, name: "bell" }, { at: 5, name: "finish" },
        ],
      };
    case "f16":
      return {
        kind,
        options: { seed: 5 },
        seconds: 4,
        controls: [
          { at: 0, name: "thrust", value: 0.5 }, { at: 0, name: "airspeed", value: 250 }, { at: 0, name: "aspect", value: -1 },
          { at: 1, name: "afterburner", value: 1, ramp: 1 }, { at: 1, name: "aspect", value: 1, ramp: 3 },
        ],
      };
    default: {
      // a tab anatomy: touch every element, glide, bow where it bows
      const touch = def.events[0];
      const first = touch.args[0].name;
      const n = touch.args[0].max;
      const fretted = touch.args.some((a) => a.name === "fret");
      const events = [];
      for (let s = 1; s <= n; s++) {
        const args = { [first]: s, velocity: 0.7 };
        if (fretted) args.fret = 2;
        events.push({ at: 0.05 + 0.25 * (s - 1), name: touch.name, args });
      }
      const glide = def.events.find((e) => e.name === "slide" || e.name === "pedal-glide");
      if (glide) events.push({ at: 2.4, name: glide.name, args: { [first]: 2, fret: 4 } });
      const controls = [];
      if (def.events.some((e) => e.name === "bow-land")) {
        events.push({ at: 2.6, name: "bow-land", args: { string: 1, speed: 0.5, force: 0.5, position: 0.12 } });
        controls.push({ at: 3.0, name: "bowSpeed", value: 0.8, ramp: 0.3 });
        events.push({ at: 3.6, name: "bow-lift" });
      }
      return { kind, seconds: 4, events, controls };
    }
  }
}
