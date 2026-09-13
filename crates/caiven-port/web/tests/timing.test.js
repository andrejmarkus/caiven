import assert from 'node:assert/strict';
import { test } from 'node:test';
import { FrameClock } from '../src/lib/caiven-timing.js';

for (const hz of [30, 60, 120, 144]) {
  test(`game runs 60 frames per second on ${hz} Hz display`, () => {
    const clock = new FrameClock();
    let frames = clock.advance(0);
    for (let i = 1; i <= hz; i++) frames += clock.advance(i * 1000 / hz);
    assert.equal(frames, 60);
  });
}

test('long stalls have bounded catch-up and reset discards hidden time', () => {
  const clock = new FrameClock();
  clock.advance(0);
  assert.equal(clock.advance(60_000), 6);
  clock.reset();
  assert.equal(clock.advance(120_000), 0);
  assert.equal(clock.advance(120_000 + 1000 / 60), 1);
});
