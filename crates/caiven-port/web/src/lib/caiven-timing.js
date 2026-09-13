// Shared by Port and Studio's offline HTML export. No browser dependencies.
export class FrameClock {
  /** @type {number | null} */
  previous = null;
  accumulator = 0;

  reset() {
    this.previous = null;
    this.accumulator = 0;
  }

  /** @param {number} now requestAnimationFrame timestamp in milliseconds */
  advance(now) {
    if (this.previous === null) {
      this.previous = now;
      return 0;
    }
    // Limit catch-up after stalls to six frames; never replay a hidden tab's gap.
    this.accumulator += Math.min(100, Math.max(0, now - this.previous));
    this.previous = now;
    const steps = Math.floor(this.accumulator * 60 / 1000 + 1e-9);
    this.accumulator = Math.max(0, this.accumulator - steps * 1000 / 60);
    return steps;
  }
}
