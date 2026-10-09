//! The tools, as MCP describes them: a name, what it does (written to teach
//! a Claude the instrument), and a JSON Schema for its arguments. Each maps
//! onto one op of the app's control socket, except `screenshot`.

use serde_json::{json, Value};

pub const INSTRUCTIONS: &str = "\
Ovillo is a bench instrument on Vera's Mac: a simulated electric guitar, played by a simulated guitarist, \
runs through Ariadne, a physical-model 'World' (a 24-mode material body that can move from a six-string \
guitar body to a B-flat brass bore, a reservoir-paid active material, ordered internal transport, and a \
shared reciprocal space), and out to the speakers. The window is a 90s instrument CRT: a spectrum or scope \
on top, the World's energies or a scrolling tab below, a configuration map / XY / chord box in the square, \
and eight softkeys on the right. These tools drive the running app through the same paths as its keys, \
so the screen, the menus and patches/live.json stay coherent. Start with get_state. Be considerate: \
someone is probably listening. set_output (mute, master_db) is the volume; it is not part of any patch. \
To compose, send play_score; to change what the guitarist improvises, use transport. Use screenshot to see.";

const PARAM_GUIDE: &str = "\
Parameters (id: range; what it does). organization 0-1: where the body sits on its configuration path, 0 = six-course \
string body tuned like the guitar, 1 = the B-flat brass bore (the MAP's x axis). source_coupling 0-4: how hard the \
guitar drives the body's compliant boundary. work_input 0-1: work per second paid into the reservoir that feeds the \
active material (its energy budget). active_coupling 0-1: how much the reservoir-paid active material takes part. \
active_drive 0-1.5: the active material's gain once awake (grows with drive squared); more drive, more \
self-sustaining, saturating growth. headroom 0.02-1: the activation threshold and the amplitudes it saturates at; \
low wakes early and limits small, high is cleaner. sustain 0-1: how much the body's own ringing keeps the active \
material awake. sag ('Hunger') 0-1: how long activation lingers after the input falls (0.1 to 1.5 s release). \
bloom ('Metabolism') 0-1: how slowly activation wakes (12 to 72 ms attack). feedback ('Reciprocity') 0-1: how much \
motion returning from the shared space re-awakens the material (with active_space). active_space 0-1: the space's \
weight in that sensing. interlace 0-1: zero-work ordered transport between near-resonant modes: beating, \
harmonic exchange, intermodulation. memory ('Path Memory') 0-1: how long that transport keeps its direction (slow). \
space_coupling ('Shared Space') 0-1: how strongly body and shared space exchange motion. space_decay \
('Persistence') 0-1: how long the shared space holds energy. space_radiation 0-1: how much the space radiates \
toward the listener. body_radiation 0-1: the fraction of the body's radiation aperture open (how directly the body \
is heard, and how fast it loses energy that way). listener_scale 0-64: gain of the stereo listener. mix 0-1: dry \
guitar to World (1 = all World). output 0-8: output gain, linear (5 = +14 dB, Ovillo's default). Slow-smoothed \
(about 20 ms): organization, feedback, active_space, memory, space_*.";

pub fn list() -> Value {
    let string_prop = json!({
        "description": "A string: 0 = low E (6th string) … 5 = high e (1st), or a name: \"E\" \"A\" \"D\" \"G\" \"B\" \"e\" (or \"E2\" … \"E4\").",
        "oneOf": [{"type": "integer", "minimum": 0, "maximum": 5}, {"type": "string"}]
    });
    let chord_prop = json!({
        "description": "A chord: a shape name (C Cmaj7 Cadd9 C7 D Dm D7 Dmaj7 Dm7 Dsus2 Dsus4 E Em E7 Em7 Esus4 Em9 Eadd9 F Fmaj7 Fmaj7#11 F#m F#m7 F#m11 G G7 Gmaj7 G6 Gsus4 G/B G#m G#m7 A Am A7 Am7 Amaj7 Asus2 Asus4 A6/9 Am(add9) Bb B Bm B7 Bm7 Bsus4 Bm(add11) C#m C#m7, power chords E5 A5 D5 G5 C5 F5 B5), six frets low E to high e ([3,2,0,0,0,3], -1/null/\"x\" for a string not played), or tab (\"x32010\", \"x-5-7-7-6-5\").",
        "oneOf": [{"type": "string"}, {"type": "array", "minItems": 6, "maxItems": 6, "items": {"type": ["integer", "null", "string"]}}]
    });
    json!([
        {
            "name": "get_state",
            "title": "Read the instrument",
            "description": "Everything on Ovillo's screen as data: the current menu and its eight softkey labels (and which is lit), the active knob function; all twenty Ariadne parameters with id, label, value, range and display text; the guitar (pickup, tone, volume, hum); the input (source live or sim, the audio interface's name and channel count, channel auto or pinned and the one in use, input gain dB, input_starved and input_dropped sample counts); the transport (composition, key it is in, tempo, playing, the loaded score and where it is, every composition with a one-line description); the display (upper spectrum/scope, lower world/tab, square map/xy/chord, map Y, time/div, amp/div, modes, hold); the master output (dB, muted); recording state and take paths; meters (in/out peak and RMS dBFS, before the master); the World's readings (material, reservoir and space energy in dB, configuration, ledger); DSP load; the guitarist's truth (each string's fret, pitch, level, whether it sounds, and the chord they spell); annunciators; pid, sample rate, audio device, socket. Call it first, and after changes to confirm them.",
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false},
            "annotations": {"readOnlyHint": true}
        },
        {
            "name": "press_keys",
            "title": "Press keys",
            "description": "Press keys on the instrument exactly as a person would, in order, through the app's own key handler (the escape hatch: anything the keyboard can do). Keys: 1-8 (or f1-f8) are the softkeys on the right bezel; escape returns to the main menu; up/down (or right/left) turn the knob, i.e. change the lit softkey's quantity, and 'shift+up' turns it finely; z x c v b n pluck the open strings low to high ('shift+z' harder); space starts/stops the guitarist; tab toggles spectrum/scope; r records a take; m mutes the strings; h toggles max hold; 0 resets the World; i switches the source between the live input and the simulated guitar (only when started with --input); - and = step the master level 1 dB (shift: 0.1 dB); \\ (or `) toggles the master mute. With a live input the SOURCE menu becomes SOURCE LIVE/SIM, IN GAIN, IN CHAN, PATTERN, TEMPO, PICKUP, MUTE, RETURN. Main menu softkeys: 1 SOURCE, 2 MATERIAL, 3 METABOLISM, 4 SPACE+OUT, 5 DISPLAY, 6 PATCH, 7 PLAY, 8 OUTPUT. Example: \"5 5 6 escape\" opens DISPLAY, switches the lower screen to the TAB, cycles the square, and returns. Returns which keys were taken, the menu now showing and its softkeys.",
            "inputSchema": {
                "type": "object",
                "properties": {"keys": {"description": "Keys separated by spaces (\"5 5 6 escape\"), or a list. Modifier: \"shift+<key>\".", "oneOf": [{"type": "string"}, {"type": "array", "items": {"type": "string"}}]}},
                "required": ["keys"], "additionalProperties": false
            }
        },
        {
            "name": "set_params",
            "title": "Set Ariadne parameters",
            "description": format!("Set any of Ariadne's twenty parameters at once (a partial patch): the others keep their values. Values are clamped to range (the reply says which). Writes patches/live.json like turning the knobs does, and the change is smoothed (no zipper noise). Keys may be the parameter id below, the live.json field name (material_organization, output_gain …), or the display name ('Hunger'). {PARAM_GUIDE}"),
            "inputSchema": {
                "type": "object",
                "properties": {"params": {"type": "object", "description": "{\"active_drive\": 0.9, \"space_decay\": 0.7, …}", "additionalProperties": {"type": "number"}}},
                "required": ["params"], "additionalProperties": false
            }
        },
        {
            "name": "get_patch",
            "title": "Read the patch",
            "description": "The current Ariadne patch exactly as patches/live.json holds it (the twenty fields the Ariadne CLAP/VST3 plug-in loads), and which of the slots A, B, C are saved.",
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false},
            "annotations": {"readOnlyHint": true}
        },
        {
            "name": "load_patch",
            "title": "Load a patch",
            "description": "Replace the whole Ariadne patch: from a saved slot (\"a\", \"b\", \"c\"), from a patch object (the twenty fields of live.json; any field left out takes the plug-in's default), or Ovillo's default patch (default: true). For changing a few parameters use set_params instead.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": {"type": "string", "enum": ["a", "b", "c"]},
                    "patch": {"type": "object", "description": "As in patches/live.json."},
                    "default": {"type": "boolean"}
                },
                "additionalProperties": false
            }
        },
        {
            "name": "save_patch",
            "title": "Save the patch to a slot",
            "description": "Save the current patch to slot a, b or c (patches/slot-a.json …), overwriting it, as the PATCH menu's SAVE keys do.",
            "inputSchema": {"type": "object", "properties": {"slot": {"type": "string", "enum": ["a", "b", "c"]}}, "required": ["slot"], "additionalProperties": false}
        },
        {
            "name": "default_patch",
            "title": "Default patch",
            "description": "Load Ovillo's default patch (patches/ovillo-default.json: the plug-in defaults with the listener and output raised so the wet signal sits near the dry level).",
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false}
        },
        {
            "name": "set_guitar",
            "title": "Set the guitar",
            "description": "Change the simulated guitar (any subset): pickup neck (warm, round), middle, bridge (bright, thin; the default) or humbucker (thick, darker, no hum); tone 0 (dark) to 1 (open); volume 0-1 (the guitar's own knob, before Ariadne; it changes how hard the World is driven); hum true/false (single-coil 60 Hz hum and noise floor).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pickup": {"type": "string", "enum": ["neck", "middle", "bridge", "humbucker"]},
                    "tone": {"type": "number", "minimum": 0, "maximum": 1},
                    "volume": {"type": "number", "minimum": 0, "maximum": 1},
                    "hum": {"type": "boolean"}
                },
                "additionalProperties": false
            }
        },
        {
            "name": "set_display",
            "title": "Set the display",
            "description": "Change what the screens show (any subset). upper: spectrum (dry blue, Ariadne orange, the body's 24 modes as green stems, the strings as ticks) or scope (triggered waveforms). lower: world (ten seconds of material/reservoir/space energy) or tab (scrolling tablature of what the guitarist plays). square: map (the configuration map: x = organization, y = map_y), xy (the output's M/S display) or chord (a chord box of the strings sounding). map_y: the map's y parameter (active_drive, work_input, active_coupling, space_coupling, interlace, sag). time_div_ms (scope: 0.5 1 2 5 10 20 50 100) and amp_div (0.005 … 0.5 FS); the nearest step is used. modes (show mode stems), hold (max hold on the spectrum), clear_hold. menu: open a softkey menu (main, source, material, metabolism, space, display, patch, output). active: make a function the knob's (tempo, tone, volume, time_div, amp_div, master, input_gain, or a parameter id), or null.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "upper": {"type": "string", "enum": ["spectrum", "scope"]},
                    "lower": {"type": "string", "enum": ["world", "tab"]},
                    "square": {"type": "string", "enum": ["map", "xy", "chord"]},
                    "map_y": {"type": "string", "enum": ["active_drive", "work_input", "active_coupling", "space_coupling", "interlace", "sag"]},
                    "time_div_ms": {"type": "number"},
                    "amp_div": {"type": "number"},
                    "modes": {"type": "boolean"},
                    "hold": {"type": "boolean"},
                    "clear_hold": {"type": "boolean"},
                    "menu": {"type": "string", "enum": ["main", "source", "material", "metabolism", "space", "display", "patch", "output"]},
                    "active": {"type": ["string", "null"]}
                },
                "additionalProperties": false
            }
        },
        {
            "name": "transport",
            "title": "The guitarist: what, how fast, play/stop",
            "description": "Choose what the simulated guitarist plays, its tempo, and start/stop it (any subset; composition first, then tempo, then play). Compositions: wander (the default; a generative chord wander through G, D, A, E and C over real voicings: Markov progressions, strummed / half-time / palm-muted drive / broken-chord / sparse sections, swells, bends, key changes; it never repeats), travis (Travis fingerpicking in C and G, alternating thumb under syncopated fingers, melodic variations), blues (12-bar shuffle in E or A: boogie, stop-time and solo choruses, licks with bends, turnarounds), ambient (open drones, high-fret plucks, slow color chords and long silences in E/A dorian or aeolian: the best for hearing Ariadne ring), melody (a slow phrased single line in A or D minor over a soft bass), the four original short loops strum, arpeggio, riff and notes, score (replay the last score sent), off. tempo: 40-220 BPM. play: false stops (what rings, rings out); true resumes the last composition.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "composition": {"type": "string", "enum": ["wander", "travis", "blues", "ambient", "melody", "strum", "arpeggio", "riff", "notes", "score", "off"]},
                    "tempo": {"type": "number", "minimum": 40, "maximum": 220},
                    "play": {"type": "boolean"}
                },
                "additionalProperties": false
            }
        },
        {
            "name": "pluck",
            "title": "Pluck a string",
            "description": "Pluck one string now, over whatever the guitarist is playing: string (0 = low E … 5 = high e), fret 0-22, velocity 0-1 (soft picks are darker), palm (palm-muted: short and dull). Optionally bend it (semitones, 0-3; 2 = a whole step) after bend_after_ms (default 80), and add vibrato (depth in semitones, 0-1; 0.15 is gentle) after vibrato_after_ms (default 250). The pluck shows on the tab and in the truth.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "string": string_prop,
                    "fret": {"type": "integer", "minimum": 0, "maximum": 22},
                    "velocity": {"type": "number", "minimum": 0, "maximum": 1},
                    "palm": {"type": "boolean"},
                    "bend": {"type": "number", "minimum": 0, "maximum": 3},
                    "bend_after_ms": {"type": "number", "minimum": 0},
                    "vibrato": {"type": "number", "minimum": 0, "maximum": 1},
                    "vibrato_after_ms": {"type": "number", "minimum": 0}
                },
                "required": ["string"], "additionalProperties": false
            }
        },
        {
            "name": "strum",
            "title": "Strum a chord",
            "description": "Strum a chord now: down (low E first) or up (high e first), velocity 0-1, spread_ms from the first string to the last (default 30; 60-120 is a slow roll). Strings the shape does not play (x) are damped. Returns the shape and the chord its notes spell.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "chord": chord_prop,
                    "dir": {"type": "string", "enum": ["down", "up"]},
                    "velocity": {"type": "number", "minimum": 0, "maximum": 1},
                    "spread_ms": {"type": "number", "minimum": 0, "maximum": 1000}
                },
                "required": ["chord"], "additionalProperties": false
            }
        },
        {
            "name": "mute_strings",
            "title": "Mute strings",
            "description": "Damp strings (as a hand laid across them): all of them, or a list. The guitarist keeps playing; this stops what rings now.",
            "inputSchema": {
                "type": "object",
                "properties": {"strings": {"description": "Omit (or \"all\") for every string, or a list of strings (0 = low E … 5 = high e, or names).", "oneOf": [{"type": "string"}, {"type": "array", "items": {"type": ["integer", "string"]}}]}},
                "additionalProperties": false
            }
        },
        {
            "name": "reset_world",
            "title": "Reset the World",
            "description": "Bring Ariadne's World to rest: body, reservoir and shared space silent, the ledger restarted. Use it if the World has been driven into a state you want to leave.",
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false}
        },
        {
            "name": "set_output",
            "title": "Master volume and mute",
            "description": "The monitor: master_db -60 to 0 (0 = unity) and mute (true/false), or toggle_mute. It sits after Ariadne, is smoothed (no clicks), is not part of any patch, and does not change what the screens and takes see. Mute when someone needs quiet; the instrument keeps playing silently.",
            "inputSchema": {
                "type": "object",
                "properties": {"master_db": {"type": "number", "minimum": -60, "maximum": 0}, "mute": {"type": "boolean"}, "toggle_mute": {"type": "boolean"}},
                "additionalProperties": false
            }
        },
        {
            "name": "record",
            "title": "Record a take",
            "description": "Start or stop a take (action: start, stop or toggle). A take is takes/take-<time>/ in the ovillo workspace: dry.wav (the guitar DI, mono), wet.wav (Ariadne, stereo, before the master) and patch.json. Returns the directory and file paths, and how long it is.",
            "inputSchema": {"type": "object", "properties": {"action": {"type": "string", "enum": ["start", "stop", "toggle"]}}, "additionalProperties": false}
        },
        {
            "name": "play_score",
            "title": "Play a score",
            "description": "Have the guitarist play music you write. A score is {\"name\", \"tempo\" (40-220 BPM; default: the current tempo), \"loop\" (default false), \"then\" (\"resume\": when a one-shot score ends the guitarist returns to the composition it was playing before, or silence if it was stopped; \"stop\": silence), \"humanize\" (0 = exact, 1 = up to 12 ms late and ±0.08 velocity), \"beats_per_bar\" (4), \"length_beats\" (default: the next bar line after the last event), \"events\": [...]}. Beats are quarter notes from 0 and may be fractional (0.5 = an eighth). Events: {\"type\": \"pluck\", \"beat\", \"string\", \"fret\" (0-22, default 0), \"velocity\" (0-1, default 0.75), \"palm\"}; {\"type\": \"strum\", \"beat\", \"chord\" (a name like \"Am7\", six frets low to high, or tab \"x02010\"), \"dir\" (down|up), \"velocity\" (default 0.8), \"spread_ms\" (first string to last, default 30), \"strings\" (only these), \"palm\"}; {\"type\": \"mute\", \"beat\", \"strings\" (default all)}; {\"type\": \"bend\", \"beat\", \"string\", \"semitones\" (0-3; 0 releases)}; {\"type\": \"vibrato\", \"beat\", \"string\", \"depth\" (0-1 semitone; 0 stops)}; {\"type\": \"rest\", \"beat\", \"beats\"} (silence that lengthens the score). Strings: 0 = low E … 5 = high e, or \"E A D G B e\". A pluck re-strikes its string (cutting what rang there) and a bend applies to the note now on that string, so put bends after their pluck. Up to 4096 events. Errors name the event and the field. Returns what was compiled: events, length, seconds.",
            "inputSchema": {
                "type": "object",
                "properties": {"score": {"type": ["object", "string"], "description": "The score object (or its JSON text)."}},
                "required": ["score"], "additionalProperties": false
            }
        },
        {
            "name": "set_input",
            "title": "Live input: source, gain, channel",
            "description": "When Ovillo was started with --input <device> (an audio interface such as the Scarlett), a real guitar can replace the simulated one as the dry signal into Ariadne. source: \"live\" (the interface) or \"sim\" (the simulated guitarist and guitar); gain_db: -24 to 36 dB on the input; channel: \"auto\" (follow the loudest input channel) or a channel number from 1. Any subset; the same as SOURCE ▸ SOURCE / IN GAIN / IN CHAN and the i key. While live, the guitarist and the simulated guitar are silent and the tab shows nothing new. On an instance started without --input this returns an error saying so. get_state's input section shows the result, with input_starved and input_dropped counts (samples the input was late with, or dropped to keep latency low).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": {"type": "string", "enum": ["live", "sim"]},
                    "gain_db": {"type": "number", "minimum": -24, "maximum": 36},
                    "channel": {"oneOf": [{"type": "string", "enum": ["auto"]}, {"type": "integer", "minimum": 1}]}
                },
                "additionalProperties": false
            }
        },
        {
            "name": "screenshot",
            "title": "Look at the screen",
            "description": "A PNG of the Ovillo window, and only that window (found by the app's pid and its own window number; other windows are never captured). Downscaled to max_width pixels (default 1240). Optionally also saved to path. Wait a moment after changes for them to be drawn (this tool already waits 150 ms).",
            "inputSchema": {
                "type": "object",
                "properties": {"max_width": {"type": "integer", "minimum": 320, "maximum": 4000}, "path": {"type": "string", "description": "Also save the PNG here."}},
                "additionalProperties": false
            },
            "annotations": {"readOnlyHint": true}
        }
    ])
}
