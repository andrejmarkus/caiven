// AudioWorkletProcessor for caiven-web cart audio. Runs in its own realm
// (no DOM, no access to the main thread's wasm Module) — the main thread
// renders PCM chunks via the caiven_audio_fill/caiven_audio_ptr wasm exports
// and posts them here for playback, so this file only ever plays back
// Float32Array chunks handed to it, never touches wasm itself.
class CaivenAudioProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.queue = [];
    this.readOffset = 0;
    this.port.onmessage = (e) => {
      this.queue.push(e.data);
    };
  }

  process(_inputs, outputs) {
    const output = outputs[0][0];
    for (let i = 0; i < output.length; i++) {
      if (this.queue.length === 0) {
        output[i] = 0;
        continue;
      }
      const chunk = this.queue[0];
      output[i] = chunk[this.readOffset++];
      if (this.readOffset >= chunk.length) {
        this.queue.shift();
        this.readOffset = 0;
      }
    }
    return true;
  }
}

registerProcessor('caiven-audio-processor', CaivenAudioProcessor);
