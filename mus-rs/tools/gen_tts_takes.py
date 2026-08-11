#!/usr/bin/env python3
"""TTS takes for the sing/chop experiments (El hilo recuerda).

Fetches vocal material from OpenAI: gpt-audio-1.5 (audio-native chat
model, follows singing instructions) and gpt-4o-mini-tts (plain chant).
The API key is read from the macOS keychain at runtime and never stored.

Run:  .venv/bin/python mus-rs/tools/gen_tts_takes.py
Writes aigua/tts_hilo_*.wav (gitignored, like every aigua wav).
"""
import base64
import json
import subprocess
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "aigua"

KEY = subprocess.run(
    ["security", "find-generic-password", "-s", "OpenAI API Key", "-a", "vera", "-w"],
    capture_output=True, text=True, check=True,
).stdout.strip()


def post(url: str, payload: dict) -> bytes:
    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode(),
        headers={"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=180) as response:
        return response.read()


def sung(name: str, voice: str, instructions: str, text: str) -> None:
    raw = post(
        "https://api.openai.com/v1/chat/completions",
        {
            "model": "gpt-audio-1.5",
            "modalities": ["text", "audio"],
            "audio": {"voice": voice, "format": "wav"},
            "messages": [
                {"role": "system", "content": instructions},
                {"role": "user", "content": text},
            ],
        },
    )
    body = json.loads(raw)
    audio_b64 = body["choices"][0]["message"]["audio"]["data"]
    out = OUT / f"{name}.wav"
    out.write_bytes(base64.b64decode(audio_b64))
    print(f"{out.name}: {out.stat().st_size} bytes ({voice})")


def spoken(name: str, voice: str, instructions: str, text: str) -> None:
    wav = post(
        "https://api.openai.com/v1/audio/speech",
        {
            "model": "gpt-4o-mini-tts-2025-12-15",
            "voice": voice,
            "response_format": "wav",
            "instructions": instructions,
            "input": text,
        },
    )
    out = OUT / f"{name}.wav"
    out.write_bytes(wav)
    print(f"{out.name}: {out.stat().st_size} bytes ({voice})")


def normalize(name: str) -> None:
    # gpt-audio WAV containers can carry a data-chunk length that is not
    # a multiple of the sample size; the engine's reader is strict, so
    # re-encode through soundfile into clean PCM_16.
    import soundfile as sf

    path = OUT / f"{name}.wav"
    audio, sr = sf.read(path)
    sf.write(path, audio, sr, subtype="PCM_16")


sung(
    "tts_hilo_canto",
    "ballad",
    "You are a singer. Sing the given words very slowly as a plainchant "
    "melisma in a minor mode: long sustained vowels, one breath per "
    "phrase, no accompaniment, no spoken framing — only the singing.",
    "El hilo recuerda el laberinto.",
)
sung(
    "tts_hilo_susurro",
    "cedar",
    "Deliver the words as a slow, half-whispered incantation, close to "
    "the microphone, long pauses between words, no spoken framing.",
    "The thread remembers the labyrinth.",
)
spoken(
    "tts_hilo_habla",
    "coral",
    "Calm, warm, unhurried. Plain speech, slight reverence.",
    "El hilo recuerda el laberinto. La sala responde en mi menor.",
)

for take in ("tts_hilo_canto", "tts_hilo_susurro", "tts_hilo_habla"):
    normalize(take)
