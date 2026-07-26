import assert from 'node:assert/strict';
import test from 'node:test';
import { TOUR_STEPS, moveTourStep } from '../src/lib/tour.ts';

test('tutorial navigation targets step being shown, not previous step', () => {
  assert.deepEqual(moveTourStep(1, 1), { index: 2, screen: 'sprites' });
  assert.deepEqual(moveTourStep(2, 1), { index: 3, screen: 'cart' });
  assert.deepEqual(moveTourStep(2, -1), { index: 1, screen: 'code' });
});

test('every tutorial step has distinct visual content and valid target', () => {
  assert.equal(TOUR_STEPS.length, 4);
  assert.equal(new Set(TOUR_STEPS.map((step) => step.id)).size, 4);
  assert.equal(new Set(TOUR_STEPS.map((step) => step.visual)).size, 4);
});
