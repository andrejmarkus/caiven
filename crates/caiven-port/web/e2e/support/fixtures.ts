import { expect, test as base } from '@playwright/test';
import { MockApi } from './mock-api';

type Fixtures = { mock: MockApi; browserGuard: void };

export const test = base.extend<Fixtures>({
  mock: async ({ page }, use) => {
    const mock = new MockApi(page);
    await mock.install();
    await use(mock);
    expect(mock.unknown, `Strict API mock received unknown calls:\n${mock.unknown.join('\n')}`).toEqual([]);
  },
  browserGuard: [async ({ page, mock }, use) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(`pageerror: ${error.message}`));
    page.on('console', (message) => {
      if (message.type() === 'warning' || message.type() === 'error') errors.push(`console.${message.type()}: ${message.text()}`);
    });
    page.on('requestfailed', (request) => {
      const url = new URL(request.url());
      const intentional = mock.allowedRequestFailures.has(`${request.method()} ${url.pathname}`)
        || mock.faults.some((fault) => fault.offline && fault.method === request.method() && fault.path === url.pathname);
      if (!intentional) errors.push(`requestfailed: ${request.method()} ${request.url()} (${request.failure()?.errorText})`);
    });
    await use();
    const unexpected = errors.filter((entry) => {
      const status = entry.match(/status of (\d+)/)?.[1];
      if (!status) return true;
      const code = Number(status);
      const remaining = mock.allowedConsoleStatuses.get(code) ?? 0;
      if (!remaining) return true;
      mock.allowedConsoleStatuses.set(code, remaining - 1);
      return false;
    });
    expect(unexpected, `Unexpected browser errors:\n${unexpected.join('\n')}`).toEqual([]);
  }, { auto: true }],
});

export { expect } from '@playwright/test';
