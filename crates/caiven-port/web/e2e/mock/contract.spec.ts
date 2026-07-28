import { readFile } from 'node:fs/promises';
import ts from 'typescript';
import { test, expect } from '../support/fixtures';
import { UI_CONTRACTS } from '../support/mock-api';

const normalize = (value: string) => value
  .replace(/\?.*$/, '')
  .replace(/<[^>]+>|:[A-Za-z][A-Za-z0-9_]*/g, ':param');
const normalizedContract = (method: string, path: string) => `${method} ${normalize(path)}`;

function frontendContracts(source: string): Set<string> {
  const file = ts.createSourceFile('api.ts', source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  const found = new Set<string>();
  function visit(node: ts.Node): void {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'request') {
      const pathArg = node.arguments[0];
      let path = '';
      if (ts.isStringLiteral(pathArg) || ts.isNoSubstitutionTemplateLiteral(pathArg)) path = pathArg.text;
      else if (ts.isTemplateExpression(pathArg)) path = pathArg.head.text + pathArg.templateSpans.map((span) => {
        const expression = span.expression.getText(file);
        return `${expression.startsWith('qs(') ? '' : ':param'}${span.literal.text}`;
      }).join('');
      let method = 'GET';
      const init = node.arguments[1];
      if (init && ts.isObjectLiteralExpression(init)) {
        const property = init.properties.find((item): item is ts.PropertyAssignment => ts.isPropertyAssignment(item) && item.name.getText(file) === 'method');
        if (property && ts.isStringLiteral(property.initializer)) method = property.initializer.text;
      }
      found.add(normalizedContract(method, `/api/v2${path}`));
    }
    ts.forEachChild(node, visit);
  }
  visit(file);
  for (const direct of [
    '/api/v2/auth/oauth/:provider/start', '/api/v2/auth/export',
    '/api/v2/carts/:id/cart', '/api/v2/carts/:id/screenshot',
  ]) found.add(normalizedContract('GET', direct));
  return found;
}

function rocketContracts(sources: string[]): Set<string> {
  const found = new Set<string>();
  const route = /#\[(get|post|put|patch|delete)\("([^"]+)"/g;
  for (const source of sources) {
    for (const match of source.matchAll(route)) found.add(normalizedContract(match[1].toUpperCase(), match[2]));
  }
  return found;
}

test('every frontend API contract exists in strict mock and Rocket', async ({}, testInfo) => {
  test.skip(testInfo.project.name !== 'desktop-chromium', 'static contract guard runs once');
  const [apiSource, ...rocketSources] = await Promise.all([
    readFile('src/api.ts', 'utf8'),
    ...['auth.rs', 'carts.rs', 'community.rs', 'discovery.rs', 'social.rs', 'versions.rs'].map((name) => readFile(`../src/handlers/${name}`, 'utf8')),
  ]);
  const frontend = frontendContracts(apiSource);
  const mock = new Set(UI_CONTRACTS.map((entry) => {
    const space = entry.indexOf(' '); return normalizedContract(entry.slice(0, space), entry.slice(space + 1));
  }));
  const rocket = rocketContracts(rocketSources);
  expect([...frontend].filter((entry) => !mock.has(entry)), 'Frontend routes missing from strict mock contract').toEqual([]);
  expect([...frontend].filter((entry) => !rocket.has(entry)), 'Frontend routes missing from Rocket handlers').toEqual([]);
});
