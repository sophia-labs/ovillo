"""Record one input device to a 48 kHz mono 24-bit WAV until SIGINT/SIGTERM or max seconds.

    python -m ovillo_voice.capture "<device name>" <out.wav> <max seconds>

CoreAudio through PortAudio (sounddevice), with a queue between the audio callback and the file, so every buffer
reaches the disk. ffmpeg's avfoundation input, which the studio used first, kept only 93% of the audio on an idle
machine and 73% on a busy one (2026-11-06): its takes have small gaps. Overflows, if any, are counted in the log.
"""
import json, queue, signal, sys, time
import sounddevice as sd
import soundfile as sf

SR = 48000


def main():
    dev, out, max_s = sys.argv[1], sys.argv[2], float(sys.argv[3])
    q, stop, over = queue.Queue(), [False], [0]

    def cb(indata, frames, t, status):
        if status.input_overflow:
            over[0] += 1
        q.put(indata.copy())

    def halt(*_):
        stop[0] = True
    signal.signal(signal.SIGINT, halt); signal.signal(signal.SIGTERM, halt)
    frames = 0
    with sf.SoundFile(out, "w", samplerate=SR, channels=1, subtype="PCM_24", format="WAV") as f, \
         sd.InputStream(device=dev, samplerate=SR, channels=1, dtype="float32", latency="high", callback=cb):
        t0 = time.time()
        while not stop[0] and time.time() - t0 < max_s:
            try:
                x = q.get(timeout=0.05)
            except queue.Empty:
                continue
            f.write(x); frames += len(x); f.flush()
        while not q.empty():
            x = q.get_nowait(); f.write(x); frames += len(x)
    print(json.dumps(dict(seconds=round(frames / SR, 3), wall=round(time.time() - t0, 3), overflows=over[0])), file=sys.stderr)


if __name__ == "__main__":
    main()
