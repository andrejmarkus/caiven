import type { Screen } from '../types';

export type TourVisual = 'code' | 'transport' | 'sprite' | 'publish';

export interface TourStep {
  id: 'write' | 'run' | 'draw' | 'ship';
  eyebrow: string;
  title: string;
  copy: string;
  screen: Screen;
  visual: TourVisual;
}

export const TOUR_STEPS: readonly TourStep[] = [
  {
    id: 'write', eyebrow: 'Write', title: 'Your cart starts here.',
    copy: 'Edit real Lua with completion, hover docs, diagnostics, and line breakpoints.',
    screen: 'code', visual: 'code',
  },
  {
    id: 'run', eyebrow: 'Run', title: 'See every change.',
    copy: 'Run, pause, reset, and step against same Rust VM used by Caiven machine.',
    screen: 'code', visual: 'transport',
  },
  {
    id: 'draw', eyebrow: 'Draw', title: 'Every pixel stays yours.',
    copy: 'Paint 8×8 sprites, maps, collision flags, palette, SFX, and music directly into cart RAM.',
    screen: 'sprites', visual: 'sprite',
  },
  {
    id: 'ship', eyebrow: 'Ship', title: 'Pack or publish.',
    copy: 'Polish cart details, export standalone .cav, or publish live buffers and cover art to port.',
    screen: 'cart', visual: 'publish',
  },
] as const;

export function moveTourStep(current: number, delta: number): { index: number; screen: Screen } {
  const index = Math.min(TOUR_STEPS.length - 1, Math.max(0, current + delta));
  return { index, screen: TOUR_STEPS[index].screen };
}
