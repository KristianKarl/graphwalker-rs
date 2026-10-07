import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import { once } from 'node:events';
import { createServer } from 'node:net';
import { readFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test, type Locator, type Page, type TestInfo } from '@playwright/test';
import type { Core } from 'cytoscape';

const E2E_DIR = dirname(fileURLToPath(import.meta.url));
const FRONTEND_DIR = resolve(E2E_DIR, '..');
const REPO_ROOT = resolve(FRONTEND_DIR, '../..');
const STATIC_DIR = resolve(E2E_DIR, '.build');
const WORKFLOW_MODEL = resolve(E2E_DIR, 'models/StudioUiWorkflow.json');
const STUDIO_BINARY = process.env.GRAPHWALKER_STUDIO_BIN
  ?? resolve(REPO_ROOT, 'target/debug/graphwalker-studio');
const GRAPHWALKER_BINARY = process.env.GRAPHWALKER_BIN
  ?? resolve(REPO_ROOT, 'target/debug/graphwalker');
const EXECUTION_SEED = '5';
const MAX_WALK_LENGTH = 100;

type FixtureVertex = { id: string; name: string; actions?: string[] };
type FixtureEdge = {
  id: string;
  name: string;
  sourceVertexId?: string;
  targetVertexId: string;
  actions?: string[];
  guard?: string;
};
type FixtureModel = {
  name: string;
  generator: string;
  startElementId?: string;
  vertices: FixtureVertex[];
  edges: FixtureEdge[];
};
const FIXTURE_MODELS = (JSON.parse(readFileSync(WORKFLOW_MODEL, 'utf8')) as {
  models: FixtureModel[];
}).models;
const WORKFLOW_MODEL_SPEC = FIXTURE_MODELS[0];
const STEP_MODEL_SPEC = FIXTURE_MODELS[1];

type Point = { x: number; y: number };
type Visit = {
  modelId: string;
  elementId: string;
  name: string;
  data: string;
  totalCount: number;
  visitedCount: number;
  fulfillment: number;
};
type StartedModel = {
  id: string;
  name: string;
  vertices: Array<{ id: string; name: string }>;
  edges: Array<{ id: string; name: string; sourceVertexId?: string; targetVertexId?: string }>;
};
type StudioMessage = {
  command?: string;
  gw?: { models?: StartedModel[] };
  modelId?: string;
  elementId?: string;
  name?: string;
  data?: string;
  totalCount?: number;
  visitedCount?: number;
  stopConditionFulfillment?: number;
  hasNext?: boolean;
};
type BrowserRun = {
  page: Page;
  testInfo: TestInfo;
  baseUrl: string;
  visits: Visit[];
  hasNextResponses: boolean[];
  fulfillmentByModel: Record<string, number>;
  startedModels: StartedModel[];
};

const FIRST_POINTS: [Point, Point, Point] = [
  { x: 150, y: 170 },
  { x: 450, y: 170 },
  { x: 300, y: 420 },
];
const SECOND_POINTS: [Point, Point, Point] = [
  { x: 160, y: 180 },
  { x: 460, y: 180 },
  { x: 310, y: 430 },
];

test('generator builder edits ordered stages and preserves complex text', async ({ page }, testInfo) => {
  const server = await startStudio();
  try {
    await page.goto(server.baseUrl);
    await page.getByRole('button', { name: 'New model' }).first().click();
    await page.getByRole('region', { name: 'Model' }).getByLabel('Name').fill('Checkout Flow');
    const execution = page.getByRole('region', { name: 'Execution' });
    const expression = execution.getByRole('textbox', { name: 'Generator expression', exact: true });
    const first = execution.getByRole('group', { name: 'Generator stage 1', exact: true });
    await expect(expression).toHaveValue('random(edge_coverage(100))');
    await first.getByLabel('Value (%)').fill('101');
    await expect(first.getByRole('alert')).toHaveText('Enter a whole number from 0 to 100.');
    await first.getByLabel('Value (%)').fill('80');
    await expect(first.getByRole('alert')).toHaveCount(0);
    await execution.getByRole('button', { name: 'Add generator', exact: true }).click();
    const second = execution.getByRole('group', { name: 'Generator stage 2', exact: true });
    await second.getByLabel('Generator', { exact: true }).selectOption('a_star');
    await expect(second.getByLabel('Stop when')).toHaveValue('reached_vertex');
    await second.getByLabel('Target name').fill('v_Checkout');
    await expect(expression).toHaveValue('random(edge_coverage(80)) a_star(reached_vertex(v_Checkout))');
    await execution.getByRole('button', { name: 'Move stage 2 up', exact: true }).click();
    await expect(expression).toHaveValue('a_star(reached_vertex(v_Checkout)) random(edge_coverage(80))');
    await execution.getByRole('button', { name: 'Move stage 1 down', exact: true }).click();
    const exported = await saveModels(page);
    expect(exported.models[0].generator).toBe('random(edge_coverage(80)) a_star(reached_vertex(v_Checkout))');
    await page.screenshot({ path: testInfo.outputPath('generator-builder-desktop.png'), animations: 'disabled' });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.screenshot({ path: testInfo.outputPath('generator-builder-mobile.png'), animations: 'disabled' });
    await expect(second.getByLabel('Target name')).toBeVisible();
    await page.setViewportSize({ width: 1440, height: 960 });
    await execution.getByRole('button', { name: 'Remove stage 2', exact: true }).click();
    await expect(execution.getByRole('button', { name: 'Remove stage 1', exact: true })).toBeDisabled();
    await first.getByLabel('Generator', { exact: true }).selectOption('new_york_street_sweeper');
    await expect(first.getByLabel('Stop when')).toHaveCount(0);
    await expect(expression).toHaveValue('new_york_street_sweeper()');
    await execution.getByRole('button', { name: 'Text', exact: true }).click();
    const text = execution.getByRole('textbox', { name: 'Generator', exact: true });
    const complex = 'random(edge_coverage(100) or time_duration(60)) a_star(reached_vertex(v_Checkout))';
    await text.fill(complex);
    await expect(execution.getByRole('button', { name: 'Builder', exact: true })).toBeDisabled();
    expect((await saveModels(page)).models[0].generator).toBe(complex);
    await text.fill('QuickRandomPath(EdgeCoverage(100)) random(length(20))');
    await execution.getByRole('button', { name: 'Builder', exact: true }).click();
    await expect(expression).toHaveValue('QuickRandomPath(EdgeCoverage(100)) random(length(20))');
    await expect(first.getByLabel('Generator', { exact: true })).toHaveValue('quick_random');
    await execution.getByRole('button', { name: 'Text', exact: true }).click();
    await text.fill('random(unknown_condition(5))');
    await expect(execution.getByRole('button', { name: 'Builder', exact: true })).toBeDisabled();
    await expect(text).toHaveValue('random(unknown_condition(5))');
  } finally {
    await stopStudio(server.child);
  }
});

test('disconnected status reconnects on click and refreshes sessions', async ({ page }) => {
  const server = await startStudio();
  let allowConnection = false;
  let requestedSessions = false;
  await page.routeWebSocket(/\/\//, (socket) => {
    if (!allowConnection) {
      socket.close();
      return;
    }
    socket.onMessage((message) => {
      const request = parseFrame(message);
      if (request?.command === 'listSessions') {
        requestedSessions = true;
        socket.send(JSON.stringify({
          command: 'sessions',
          success: true,
          sessions: [{ id: 'retry-session', name: 'Retry Session' }],
        }));
      }
    });
  });

  try {
    await page.goto(server.baseUrl);
    const connection = page.getByRole('button', { name: 'Disconnected', exact: true });
    await expect(connection).toBeEnabled();
    allowConnection = true;
    await connection.click();
    await expect(page.getByRole('button', { name: 'Connected', exact: true })).toBeDisabled();
    await expect.poll(() => requestedSessions).toBe(true);
    await expect(page.getByText('Sessions (1)', { exact: true })).toBeVisible();
  } finally {
    await stopStudio(server.child);
  }
});

test('graph context menu toggles breakpoints for every element', async ({ page }) => {
  const server = await startStudio();

  try {
    await page.goto(server.baseUrl);
    await expect(page.getByText('Connected', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'New model' }).first().click();
    const graph = page.getByRole('application').first();
    await addCycle(page, graph, FIRST_POINTS);

    const bounds = await graph.boundingBox();
    if (!bounds) throw new Error('Graph editor has no visible bounds');
    const openCanvasMenu = () => page.mouse.click(
      bounds.x + bounds.width - 20,
      bounds.y + bounds.height - 20,
      { button: 'right' },
    );

    await openCanvasMenu();
    await page.getByRole('button', { name: /Set breakpoints on all elements/ }).click();
    await graph.click({ position: { x: 600, y: 500 } });

    const readBreakpointStyles = () => graph.evaluate((container) => {
      const canvas = container.firstElementChild as HTMLElement & { _cyreg: { cy: Core } };
      const cy = canvas._cyreg.cy;
      return {
        nodes: cy.nodes('.breakpoint').map((node) => ({
          color: node.style('border-color'),
          outline: node.style('border-style'),
        })),
        edges: cy.edges('.breakpoint').map((edge) => ({
          color: edge.style('line-color'),
          arrow: edge.style('target-arrow-color'),
          outline: edge.style('line-style'),
        })),
      };
    });
    const expectedStyles = {
      nodes: Array.from({ length: 3 }, () => ({ color: 'rgb(239,68,68)', outline: 'dashed' })),
      edges: Array.from({ length: 3 }, () => ({
        color: 'rgb(239,68,68)', arrow: 'rgb(239,68,68)', outline: 'dashed',
      })),
    };
    await expect.poll(readBreakpointStyles).toEqual(expectedStyles);
    await page.getByRole('button', { name: 'Switch to the light color theme.' }).click();
    await expect.poll(readBreakpointStyles).toEqual(expectedStyles);
    await page.getByRole('button', { name: 'Switch to the dark color theme.' }).click();
    await expect.poll(readBreakpointStyles).toEqual(expectedStyles);

    await openCanvasMenu();
    await page.getByRole('button', { name: /Clear all breakpoints/ }).click();
    await expect.poll(readBreakpointStyles).toEqual({ nodes: [], edges: [] });
    await openCanvasMenu();
    await expect(page.getByRole('button', { name: /Set breakpoints on all elements/ }))
      .toBeVisible();
  } finally {
    await stopStudio(server.child);
  }
});

test('Stop resets a fully executed session', async ({ page }) => {
  const server = await startStudio();
  let exhausted = false;
  page.on('websocket', (socket) => {
    socket.on('framereceived', ({ payload }) => {
      const message = parseFrame(payload);
      if (message?.command === 'hasNext' && message.hasNext === false) exhausted = true;
    });
  });

  try {
    await page.goto(server.baseUrl);
    await expect(page.getByText('Connected', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'New model' }).first().click();
    const graph = page.getByRole('application').first();
    await addCycle(page, graph, FIRST_POINTS);
    await selectPoint(graph, FIRST_POINTS[0]);
    await page.getByRole('region', { name: 'Element' }).getByRole('button', {
      name: 'Set as start',
    }).click();
    await page.getByRole('region', { name: 'Execution' }).getByRole('button', { name: 'Text', exact: true }).click();
    await page.getByRole('region', { name: 'Execution' }).getByLabel('Generator', { exact: true })
      .fill('random(length(3))');

    const stop = page.getByRole('button', {
      name: 'Stop the current walk or leave the observed session and clear its progress.',
    });
    const readProgress = () => graph.evaluate((container) => {
      const canvas = container.firstElementChild as HTMLElement & { _cyreg: { cy: Core } };
      return canvas._cyreg.cy.$('.visited, .current').length;
    });
    await expect(stop).toBeDisabled();
    await page.getByRole('button', {
      name: 'Start or resume walking the model using its selected generator.',
    }).click();
    await expect.poll(() => exhausted).toBe(true);
    await expect(page.getByText('Ready', { exact: true })).toBeVisible();
    await expect.poll(readProgress).toBeGreaterThan(0);
    await expect(stop).toBeEnabled();
    await stop.click();
    await expect.poll(readProgress).toBe(0);
    await expect(stop).toBeDisabled();
  } finally {
    await stopStudio(server.child);
  }
});

test('theme switches preserve visited graph colors', async ({ page }) => {
  const server = await startStudio();

  try {
    await page.goto(server.baseUrl);
    await expect(page.getByText('Connected', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'New model' }).first().click();
    const graph = page.getByRole('application').first();
    await addCycle(page, graph, FIRST_POINTS);
    await selectPoint(graph, FIRST_POINTS[0]);
    await page.getByRole('region', { name: 'Element' }).getByRole('button', {
      name: 'Set as start',
    }).click();
    await graph.click({ position: { x: 600, y: 500 } });

    const readVisitedStyles = () => graph.evaluate((container) => {
      const canvas = container.firstElementChild as HTMLElement & { _cyreg: { cy: Core } };
      const cy = canvas._cyreg.cy;
      return {
        nodes: cy.nodes('.visited').map((node) => node.style('background-color')),
        edges: cy.edges('.visited').map((edge) => edge.style('line-color')),
        current: cy.$('.current').map((element) => element.id()),
      };
    });

    const step = page.getByRole('button', { name: 'Step' });
    await step.click();
    await expect.poll(async () => (await readVisitedStyles()).nodes.length).toBe(1);
    await step.click();
    await expect.poll(async () => (await readVisitedStyles()).edges.length).toBe(1);
    const before = await readVisitedStyles();
    expect(before.nodes).toEqual(['rgb(42,80,58)']);
    expect(before.edges).toEqual(['rgb(34,197,94)']);
    expect(before.current).toHaveLength(1);

    await page.getByRole('button', { name: 'Switch to the light color theme.' }).click();
    await expect.poll(readVisitedStyles).toEqual({
      ...before,
      nodes: ['rgb(212,237,218)'],
    });
    await page.getByRole('button', { name: 'Switch to the dark color theme.' }).click();
    await expect.poll(readVisitedStyles).toEqual(before);
  } finally {
    await stopStudio(server.child);
  }
});

test('Studio UI supports authoring and running multiple models', async ({ page }, testInfo) => {
  const server = await startStudio();
  const run: BrowserRun = {
    page,
    testInfo,
    baseUrl: server.baseUrl,
    visits: [],
    hasNextResponses: [],
    fulfillmentByModel: {},
    startedModels: [],
  };

  page.on('websocket', (socket) => {
    socket.on('framesent', ({ payload }) => {
      const message = parseFrame(payload);
      if (message?.command === 'start' && Array.isArray(message.gw?.models)) {
        run.startedModels = message.gw.models;
      }
    });
    socket.on('framereceived', ({ payload }) => {
      const message = parseFrame(payload);
      if (message?.command === 'hasNext' && typeof message.hasNext === 'boolean') {
        run.hasNextResponses.push(message.hasNext);
      }
      if (
        message?.command === 'visitedElement'
        && typeof message.modelId === 'string'
        && typeof message.elementId === 'string'
        && typeof message.name === 'string'
        && typeof message.data === 'string'
        && typeof message.totalCount === 'number'
        && typeof message.visitedCount === 'number'
        && typeof message.stopConditionFulfillment === 'number'
      ) {
        const visit: Visit = {
          modelId: message.modelId,
          elementId: message.elementId,
          name: message.name,
          data: message.data,
          totalCount: message.totalCount,
          visitedCount: message.visitedCount,
          fulfillment: message.stopConditionFulfillment,
        };
        run.visits.push(visit);
        run.fulfillmentByModel[visit.modelId] = visit.fulfillment;
      }
    });
  });

  try {
    const scenarioPath = generateScenarioPath();
    const actions: Record<string, () => Promise<void>> = {
      e_openStudio: () => openStudio(run),
      e_createFirstModel: () => createFirstModel(run),
      e_renameFirstModel: () => renameFirstModel(run),
      e_buildFirstGraph: () => buildFirstGraph(run),
      e_editFirstModel: () => editFirstModel(run),
      e_finishEditingFirstModel: () => finishEditingFirstModel(run),
      e_createSecondModel: () => createSecondModel(run),
      e_switchModels: () => switchModelsAndExport(run),
      e_runModels: () => runBothModels(run),
      e_editSecondModel: () => editSecondModel(run),
      e_stepSecondModel: () => stepSecondModelToExhaustion(run),
      e_deleteElement: () => deleteSecondModelElement(run),
      e_closeSecondModel: () => closeSecondModel(run),
      e_closeAllModels: () => closeAllModels(run),
    };
    const executed = new Set<string>();

    for (const edgeName of scenarioPath) {
      const action = actions[edgeName];
      if (!action) throw new Error(`No Playwright scenario registered for ${edgeName}`);
      await action();
      executed.add(edgeName);
    }

    expect([...executed].sort()).toEqual(Object.keys(actions).sort());
    await finish(run, testInfo);
  } finally {
    await stopStudio(server.child);
  }
});

async function openStudio(run: BrowserRun) {
  await run.page.goto(run.baseUrl);
  await expect(run.page.getByText('Connected', { exact: true })).toBeVisible();
  await expect(run.page.getByRole('button', { name: 'New model' }).first()).toBeVisible();
  await run.page.getByRole('button', { name: 'Toggle theme' }).click();
}

async function createFirstModel(run: BrowserRun) {
  await run.page.getByRole('button', { name: 'New model' }).first().click();
  await run.page.getByRole('region', { name: 'Model' }).getByLabel('Name').fill('Checkout Flow');
  await expect(run.page.getByRole('tab', { name: 'Checkout Flow' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
}

async function renameFirstModel(run: BrowserRun) {
  const name = run.page.getByRole('region', { name: 'Model' }).getByLabel('Name');
  await name.fill('Checkout Draft');
  await expect(run.page.getByRole('tab', { name: 'Checkout Draft' })).toBeVisible();
  await name.fill('Checkout Flow');
  await expect(run.page.getByRole('tab', { name: 'Checkout Flow' })).toBeVisible();
}

async function buildFirstGraph(run: BrowserRun) {
  const graph = run.page.getByRole('application', { name: 'Graph editor for Checkout Flow' });
  await addCycle(run.page, graph, FIRST_POINTS);
  await selectPoint(graph, FIRST_POINTS[0]);
  await run.page.getByRole('region', { name: 'Element' }).getByRole('button', {
    name: 'Set as start',
  }).click();
  await expect(run.page.getByRole('region', { name: 'Element' }).getByRole('button', {
    name: 'Start',
  })).toBeVisible();
  await expect(run.page.getByText('Model OK', { exact: true })).toBeVisible();
  await captureScreenshot(run.page, 'first-model-created.png', run.testInfo);
}

async function editFirstModel(run: BrowserRun) {
  await run.page.getByRole('region', { name: 'Global' }).getByLabel('Global data').fill('counter = 0;');
  const graph = run.page.getByRole('application', { name: 'Graph editor for Checkout Flow' });
  await selectPoint(graph, FIRST_POINTS[0]);
  await run.page.getByRole('region', { name: 'Element' }).getByLabel('Name').fill('v_OrderReady');

  await selectPoint(graph, midpoint(FIRST_POINTS[0], FIRST_POINTS[1]));
  const element = run.page.getByRole('region', { name: 'Element' });
  await element.getByLabel('Name').fill('e_SubmitOrder');
  await element.getByLabel('Guard').fill('true');
  await element.getByLabel('Actions').fill('counter = 1;');

  const execution = run.page.getByRole('region', { name: 'Execution' });
  await execution.getByRole('button', { name: 'Text', exact: true }).click();
  await execution.getByLabel('Generator', { exact: true }).fill('random(vertex_coverage(100))');
  await expect(execution.getByLabel('Generator', { exact: true })).toHaveValue('random(vertex_coverage(100))');
  await expect(run.page.getByText('Model OK', { exact: true })).toBeVisible();

  const exported = await saveModels(run.page);
  expect(exported.models[0].vertices).toContainEqual(
    expect.objectContaining({ name: 'v_OrderReady' }),
  );
  expect(exported.models[0].edges).toContainEqual(
    expect.objectContaining({ name: 'e_SubmitOrder', guard: 'true', actions: ['counter = 1;'] }),
  );
  expect(exported.models[0].generator).toBe('random(vertex_coverage(100))');
}

async function finishEditingFirstModel(run: BrowserRun) {
  await expect(run.page.getByRole('tab', { name: 'Checkout Flow' })).toBeVisible();
  await expect(run.page.getByRole('region', { name: 'Model' }).getByLabel('Name'))
    .toHaveValue('Checkout Flow');
  await expect(run.page.getByText('Model OK', { exact: true })).toBeVisible();
}

async function createSecondModel(run: BrowserRun) {
  await run.page.getByRole('button', { name: 'New model' }).first().click();
  await run.page.getByRole('region', { name: 'Model' }).getByLabel('Name').fill(STEP_MODEL_SPEC.name);
  const graph = run.page.getByRole('application', {
    name: `Graph editor for ${STEP_MODEL_SPEC.name}`,
  });
  await addCycle(run.page, graph, SECOND_POINTS);
  const element = run.page.getByRole('region', { name: 'Element' });
  for (let index = 0; index < STEP_MODEL_SPEC.vertices.length; index += 1) {
    await selectPoint(graph, SECOND_POINTS[index]);
    await element.getByLabel('Name').fill(STEP_MODEL_SPEC.vertices[index].name);
    await element.getByLabel('Actions').fill(
      STEP_MODEL_SPEC.vertices[index].actions?.join('\n') ?? '',
    );
  }

  const startIndex = STEP_MODEL_SPEC.vertices.findIndex(
    (vertex) => vertex.id === STEP_MODEL_SPEC.startElementId,
  );
  expect(startIndex).toBeGreaterThanOrEqual(0);
  await selectPoint(graph, SECOND_POINTS[startIndex]);
  await element.getByRole('button', {
    name: 'Set as start',
  }).click();

  for (const edge of STEP_MODEL_SPEC.edges) {
    const sourceIndex = STEP_MODEL_SPEC.vertices.findIndex(
      (vertex) => vertex.id === edge.sourceVertexId,
    );
    const targetIndex = STEP_MODEL_SPEC.vertices.findIndex(
      (vertex) => vertex.id === edge.targetVertexId,
    );
    expect(sourceIndex).toBeGreaterThanOrEqual(0);
    expect(targetIndex).toBeGreaterThanOrEqual(0);
    const source = SECOND_POINTS[sourceIndex];
    const target = SECOND_POINTS[targetIndex];
    await selectPoint(graph, midpoint(source, target));
    await element.getByLabel('Name').fill(edge.name);
    await element.getByLabel('Actions').fill(edge.actions?.join('\n') ?? '');
    await element.getByLabel('Guard').fill(edge.guard ?? '');
  }
  await run.page.getByRole('region', { name: 'Execution' }).getByRole('button', { name: 'Text', exact: true }).click();
  await run.page.getByRole('region', { name: 'Execution' }).getByLabel('Generator', { exact: true })
    .fill(STEP_MODEL_SPEC.generator);
  await expect(run.page.getByRole('tab', { name: 'Checkout Flow' })).toBeVisible();
  await expect(run.page.getByRole('tab', { name: STEP_MODEL_SPEC.name })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  await expect(run.page.getByText('Model OK', { exact: true })).toBeVisible();
}

async function editSecondModel(run: BrowserRun) {
  await run.page.getByRole('tab', { name: STEP_MODEL_SPEC.name }).click();
  const graph = run.page.getByRole('application', {
    name: `Graph editor for ${STEP_MODEL_SPEC.name}`,
  });
  const edge = STEP_MODEL_SPEC.edges[0];
  const sourceIndex = STEP_MODEL_SPEC.vertices.findIndex(
    (vertex) => vertex.id === edge.sourceVertexId,
  );
  const targetIndex = STEP_MODEL_SPEC.vertices.findIndex(
    (vertex) => vertex.id === edge.targetVertexId,
  );
  await selectPoint(graph, midpoint(SECOND_POINTS[sourceIndex], SECOND_POINTS[targetIndex]));
  const nameField = run.page.getByRole('region', { name: 'Element' }).getByLabel('Name');
  await nameField.fill(`${edge.name}Draft`);
  await expect(nameField).toHaveValue(`${edge.name}Draft`);
  await nameField.fill(edge.name);
  await expect(nameField).toHaveValue(edge.name);
  await expect(run.page.getByRole('tab', { name: STEP_MODEL_SPEC.name })).toBeVisible();
}

async function switchModelsAndExport(run: BrowserRun) {
  const firstTab = run.page.getByRole('tab', { name: 'Checkout Flow' });
  const secondTab = run.page.getByRole('tab', { name: 'Refund Flow' });
  await firstTab.click();
  await expect(firstTab).toHaveAttribute('aria-selected', 'true');
  const firstName = run.page.getByRole('region', { name: 'Model' }).getByLabel('Name');
  await run.page.getByRole('region', { name: 'Execution' }).getByRole('button', { name: 'Text', exact: true }).click();
  const firstGenerator = run.page.getByRole('region', { name: 'Execution' }).getByLabel('Generator', { exact: true });
  await expect(firstName).toHaveValue('Checkout Flow');
  const firstGeneratorValue = await firstGenerator.inputValue();
  expect(['random(edge_coverage(100))', 'random(vertex_coverage(100))'])
    .toContain(firstGeneratorValue);

  await secondTab.click();
  await expect(secondTab).toHaveAttribute('aria-selected', 'true');
  await expect(run.page.getByRole('region', { name: 'Model' }).getByLabel('Name'))
    .toHaveValue('Refund Flow');
  await run.page.getByRole('region', { name: 'Execution' }).getByRole('button', { name: 'Text', exact: true }).click();
  await expect(run.page.getByRole('region', { name: 'Execution' }).getByLabel('Generator', { exact: true }))
    .toHaveValue('random(edge_coverage(100))');

  await firstTab.click();
  await run.page.getByRole('region', { name: 'Execution' }).getByRole('button', { name: 'Text', exact: true }).click();
  await expect(firstName).toHaveValue('Checkout Flow');
  await expect(firstGenerator).toHaveValue(firstGeneratorValue);
  await run.page.getByRole('region', { name: 'Global' }).getByLabel('Auto').uncheck();
  await run.page.getByRole('region', { name: 'Global' }).getByLabel('Seed').fill(EXECUTION_SEED);

  const exported = await saveModels(run.page);
  expect(exported.seed).toBe(Number(EXECUTION_SEED));
  expect(exported.models.map((model) => model.name)).toEqual(['Checkout Flow', 'Refund Flow']);
  expect(exported.models[0].vertices).toHaveLength(3);
  expect(exported.models[0].edges).toHaveLength(3);
  expect(exported.models[1].vertices).toHaveLength(3);
  expect(exported.models[1].edges).toHaveLength(3);
}

async function saveModels(page: Page) {
  const downloadPromise = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Save' }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe('Checkout_Flow.json');
  const savedFile = await download.path();
  expect(savedFile).not.toBeNull();
  return JSON.parse(await readFile(savedFile!, 'utf8')) as {
    seed?: number;
    models: Array<{
      name: string;
      generator: string;
      vertices: Array<{ name: string }>;
      edges: Array<{ name: string; guard?: string; actions?: string[] }>;
    }>;
  };
}

async function runBothModels(run: BrowserRun) {
  resetExecutionObservations(run);
  const delay = run.page.getByRole('slider', { name: 'Step delay' });
  await delay.focus();
  await delay.press('Home');
  for (let index = 0; index < 20; index += 1) await delay.press('ArrowRight');
  await expect(run.page.getByText('Step delay: 200ms', { exact: true })).toBeVisible();

  await run.page.getByRole('button', { name: 'Play' }).click();
  await expect(run.page.getByRole('button', { name: 'Pause' })).toBeVisible();
  await expect.poll(() => run.visits.length).toBeGreaterThan(0);
  await run.page.getByRole('button', { name: 'Pause' }).click();
  await expect(run.page.getByText('Paused', { exact: true })).toBeVisible();
  await expectStableVisitCount(run, run.visits.length);

  const visitCountBeforeStep = run.visits.length;
  await run.page.getByRole('button', { name: 'Step' }).click();
  await expect.poll(() => run.visits.length).toBe(visitCountBeforeStep + 1);

  await run.page.getByRole('button', { name: 'Play' }).click();
  await expect(run.page.getByText('Ready', { exact: true })).toBeVisible({ timeout: 30_000 });
  expect(run.startedModels).toHaveLength(2);
  const modelIds = new Set(run.startedModels.map((model) => model.id));
  expect(new Set(run.visits.map((visit) => visit.modelId))).toEqual(modelIds);
  await captureScreenshot(run.page, 'both-models-executed.png', run.testInfo);
}

async function stepSecondModelToExhaustion(run: BrowserRun) {
  const globalData = run.page.getByRole('region', { name: 'Global' }).getByLabel('Global data');
  await globalData.fill('counter = 0;');
  resetExecutionObservations(run);

  const stepButton = run.page.getByRole('button', { name: 'Step' });
  let exhausted = false;
  for (let stepNumber = 1; stepNumber <= 100; stepNumber += 1) {
    const previousVisitCount = run.visits.length;
    const previousHasNextCount = run.hasNextResponses.length;
    await stepButton.click();
    await expect.poll(() => run.hasNextResponses.length).toBeGreaterThan(previousHasNextCount);
    const hasNext = run.hasNextResponses.at(-1);

    if (!hasNext) {
      expect(run.visits).toHaveLength(previousVisitCount);
      exhausted = true;
      break;
    }

    await expect.poll(() => run.visits.length).toBe(previousVisitCount + 1);
    const visit = run.visits[previousVisitCount];
    await assertStepVisibleInStudio(run, visit, stepNumber);
  }

  expect(exhausted, 'Step did not report hasNext=false within 100 steps').toBe(true);
  expect(run.startedModels).toHaveLength(2);
  const secondModel = run.startedModels.find((model) => model.name === STEP_MODEL_SPEC.name);
  expect(secondModel, 'Refund Flow was not included in the stepped session').toBeDefined();
  expect(run.visits.some((visit) => visit.modelId === secondModel!.id)).toBe(true);
  for (const edge of STEP_MODEL_SPEC.edges) {
    expect(
      run.visits.some((visit) => visit.modelId === secondModel!.id && visit.name === edge.name),
      `${STEP_MODEL_SPEC.name} edge ${edge.name} was never stepped`,
    ).toBe(true);
  }
  expect(run.fulfillmentByModel[secondModel!.id]).toBeGreaterThanOrEqual(0.999999);

  await run.page.getByRole('button', { name: 'Stop' }).click();
  await expect(run.page.getByText('Ready', { exact: true })).toBeVisible();
  await captureScreenshot(run.page, 'refund-flow-stepped-to-exhaustion.png', run.testInfo);
}

function resetExecutionObservations(run: BrowserRun) {
  run.visits = [];
  run.hasNextResponses = [];
  run.fulfillmentByModel = {};
  run.startedModels = [];
}

async function assertStepVisibleInStudio(run: BrowserRun, visit: Visit, stepNumber: number) {
  const model = run.startedModels.find((entry) => entry.id === visit.modelId);
  expect(model, `Visited an unknown model ${visit.modelId}`).toBeDefined();
  const graph = run.page.getByRole('application', { name: `Graph editor for ${model!.name}` });
  const tab = run.page.getByRole('tab', { name: model!.name });
  await expect(tab).toHaveAttribute('aria-selected', 'true');
  await expect(run.page.getByRole('region', { name: 'Element' }).getByLabel('Name'))
    .toHaveValue(visit.name);

  const currentTotal = model!.vertices.length
    + model!.edges.filter((edge) => edge.sourceVertexId).length;
  const allVertices = run.startedModels.reduce((sum, item) => sum + item.vertices.length, 0);
  const allEdges = run.startedModels.reduce(
    (sum, item) => sum + item.edges.filter((edge) => edge.sourceVertexId).length,
    0,
  );
  const allTotal = allVertices + allEdges;
  const visitedIdsForModel = new Set(
    run.visits.filter((item) => item.modelId === model!.id).map((item) => item.elementId),
  );
  const currentElementIds = new Set([
    ...model!.vertices.map((vertex) => vertex.id),
    ...model!.edges.filter((edge) => edge.sourceVertexId).map((edge) => edge.id),
  ]);
  const currentVisited = [...visitedIdsForModel].filter((id) => currentElementIds.has(id)).length;
  const allElementIds = new Set(
    run.startedModels.flatMap((item) => [
      ...item.vertices.map((vertex) => vertex.id),
      ...item.edges.filter((edge) => edge.sourceVertexId).map((edge) => edge.id),
    ]),
  );
  const allVisitedIds = new Set(run.visits.map((item) => item.elementId));
  const allVisited = [...allVisitedIds].filter((id) => allElementIds.has(id)).length;
  const currentEdges = model!.edges.filter((edge) => edge.sourceVertexId).length;
  const unvisitedAll = allTotal - allVisited;
  const unvisitedModel = currentTotal - currentVisited;

  await expect(graph.getByRole('row', {
    name: `Vertices ${model!.vertices.length} (${allVertices})`,
  })).toBeVisible();
  await expect(graph.getByRole('row', { name: `Edges ${currentEdges} (${allEdges})` })).toBeVisible();
  await expect(graph.getByRole('row', { name: `Steps ${stepNumber}` })).toBeVisible();
  await expect(graph.getByRole('row', { name: `Unvisited (all) ${unvisitedAll}` })).toBeVisible();
  await expect(graph.getByRole('row', { name: `Unvisited (model) ${unvisitedModel}` }))
    .toBeVisible();

  const counter = /(?:^|;)global\.counter=(-?\d+)/.exec(visit.data);
  expect(counter, `Step data did not contain global.counter: ${visit.data}`).not.toBeNull();
  await expect(graph.getByText('Model Data', { exact: true })).toBeVisible();
  await expect(graph.getByRole('row', {
    name: `global.counter ${counter![1]}`,
  })).toBeVisible();

  const percent = Math.round(
    Object.values(run.fulfillmentByModel).reduce((sum, value) => sum + value, 0)
      / Object.keys(run.fulfillmentByModel).length
      * 100,
  );
  await expect(run.page.getByText(`${percent}%`, { exact: true })).toBeVisible();
  await expect(run.page.getByText('Paused', { exact: true })).toBeVisible();
  await captureScreenshot(run.page, `step-${stepNumber}-${model!.name}.png`, run.testInfo);
}

async function expectStableVisitCount(run: BrowserRun, expected: number) {
  let stablePolls = 0;
  await expect.poll(() => {
    stablePolls = run.visits.length === expected ? stablePolls + 1 : 0;
    return stablePolls;
  }, { intervals: [100, 100, 100, 100, 100], timeout: 3_000 }).toBe(4);
}

async function deleteSecondModelElement(run: BrowserRun) {
  await run.page.getByRole('tab', { name: STEP_MODEL_SPEC.name }).click();
  const graph = run.page.getByRole('application', {
    name: `Graph editor for ${STEP_MODEL_SPEC.name}`,
  });
  await selectPoint(graph, SECOND_POINTS[1]);
  await run.page.keyboard.press('Delete');
  await expect(graph.getByText('2 (5)', { exact: true })).toBeVisible();
  await expect(graph.getByText('1 (4)', { exact: true })).toBeVisible();
  await expect(run.page.getByText('Model OK', { exact: true })).not.toBeVisible();
}

async function closeSecondModel(run: BrowserRun) {
  await run.page.getByRole('button', { name: `Close ${STEP_MODEL_SPEC.name}` }).click();
  await expect(run.page.getByRole('tab', { name: STEP_MODEL_SPEC.name })).toHaveCount(0);
  await expect(run.page.getByRole('tab', { name: 'Checkout Flow' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  await expect(run.page.getByRole('region', { name: 'Model' }).getByLabel('Name'))
    .toHaveValue('Checkout Flow');
  await expect(run.page.getByRole('application', { name: 'Graph editor for Checkout Flow' }))
    .toBeVisible();
}

async function closeAllModels(run: BrowserRun) {
  const tabs = run.page.getByRole('tab');
  while (await tabs.count() > 0) {
    const name = (await tabs.first().innerText()).trim();
    await run.page.getByRole('button', { name: `Close ${name}` }).click();
  }
  await expect(tabs).toHaveCount(0);
  await expect(run.page.getByText('No model selected', { exact: true })).toBeVisible();
}

async function finish(run: BrowserRun, testInfo: TestInfo) {
  await expect(run.page.getByText('Ready', { exact: true })).toBeVisible();
  if (await run.page.getByRole('tab').count() > 0) {
    await expect(run.page.getByText('Model OK', { exact: true })).toBeVisible();
  } else {
    await expect(run.page.getByRole('button', { name: 'New model' }).first()).toBeVisible();
  }
  await captureScreenshot(run.page, 'studio-ui-finished.png', testInfo);
}

async function addCycle(page: Page, graph: Locator, points: [Point, Point, Point]) {
  await graph.focus();
  for (const point of points) {
    await page.keyboard.down('v');
    try {
      await graph.click({ position: point });
    } finally {
      await page.keyboard.up('v');
    }
  }
  await addEdge(page, graph, points[0], points[1]);
  await addEdge(page, graph, points[1], points[2]);
  await addEdge(page, graph, points[2], points[0]);
}

async function addEdge(page: Page, graph: Locator, source: Point, target: Point) {
  const bounds = await graph.boundingBox();
  if (!bounds) throw new Error('Graph editor has no visible bounds');
  await page.keyboard.down('e');
  try {
    await page.mouse.move(bounds.x + source.x, bounds.y + source.y);
    await page.mouse.down();
    await page.mouse.move(bounds.x + target.x, bounds.y + target.y, { steps: 6 });
    await page.mouse.up();
  } finally {
    await page.keyboard.up('e');
  }
}

async function selectPoint(graph: Locator, point: Point) {
  await graph.click({ position: point });
}

function midpoint(a: Point, b: Point): Point {
  return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
}

function generateScenarioPath(): string[] {
  const output = execFileSync(
    GRAPHWALKER_BINARY,
    ['offline', '-o', '-g', WORKFLOW_MODEL, '--seed', EXECUTION_SEED],
    { cwd: REPO_ROOT, encoding: 'utf8', timeout: 15_000 },
  );
  const pathSteps = output
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line) as { currentElementId?: string; currentElementName?: string });
  const workflowEdgeIds = new Set(WORKFLOW_MODEL_SPEC.edges.map((edge) => edge.id));
  const workflowEdgeSteps = pathSteps.filter((step) => workflowEdgeIds.has(step.currentElementId ?? ''));
  const workflowNames = workflowEdgeSteps.map((step) => step.currentElementName ?? '');
  const missingScenarios = WORKFLOW_MODEL_SPEC.edges
    .map((edge) => edge.name)
    .filter((name) => !workflowNames.includes(name));
  const missingStepEdges = STEP_MODEL_SPEC.edges.filter(
    (edge) => !pathSteps.some((step) => step.currentElementId === edge.id),
  );
  if (
    pathSteps.length > MAX_WALK_LENGTH
    || missingScenarios.length > 0
    || missingStepEdges.length > 0
  ) {
    throw new Error(
      `GraphWalker path failed fixture coverage (missing workflow: ${missingScenarios.join(', ')}; `
        + `missing step edges: ${missingStepEdges.map((edge) => edge.id).join(', ')}; `
        + `path steps: ${pathSteps.length})`,
    );
  }
  return workflowEdgeSteps.map((step) => step.currentElementName ?? '');
}

function parseFrame(payload: string | Buffer): StudioMessage | undefined {
  try {
    return JSON.parse(typeof payload === 'string' ? payload : payload.toString('utf8')) as StudioMessage;
  } catch {
    return undefined;
  }
}

async function captureScreenshot(page: Page, name: string, testInfo: TestInfo) {
  await testInfo.attach(name, {
    body: await page.screenshot({ animations: 'disabled' }),
    contentType: 'image/png',
  });
}

async function availablePort(): Promise<number> {
  const server = createServer();
  await new Promise<void>((resolveListen, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolveListen);
  });
  const address = server.address();
  if (!address || typeof address === 'string') throw new Error('Failed to reserve local port');
  const port = address.port;
  await new Promise<void>((resolveClose, reject) => {
    server.close((error) => error ? reject(error) : resolveClose());
  });
  return port;
}

async function startStudio(): Promise<{ child: ChildProcess; baseUrl: string }> {
  const browserPort = await availablePort();
  let websocketPort = await availablePort();
  while (websocketPort === browserPort) websocketPort = await availablePort();
  const child = spawn(
    STUDIO_BINARY,
    [
      '--browser-port', String(browserPort),
      '--websocket-port', String(websocketPort),
      '--static-dir', STATIC_DIR,
    ],
    { cwd: REPO_ROOT, stdio: ['ignore', 'pipe', 'pipe'] },
  );
  let logs = '';
  child.stdout?.on('data', (chunk: Buffer) => { logs += chunk.toString(); });
  child.stderr?.on('data', (chunk: Buffer) => { logs += chunk.toString(); });
  const baseUrl = `http://127.0.0.1:${browserPort}`;

  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (child.exitCode !== null) {
      throw new Error(`Studio exited during startup (${child.exitCode}):\n${logs}`);
    }
    try {
      const response = await fetch(baseUrl, { signal: AbortSignal.timeout(500) });
      if (response.ok) return { child, baseUrl };
    } catch {
      // Retry until the HTTP listener is ready.
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 100));
  }
  await stopStudio(child);
  throw new Error(`Studio did not become ready:\n${logs}`);
}

async function stopStudio(child: ChildProcess) {
  if (child.exitCode !== null || child.signalCode !== null) return;
  const gracefulExit = once(child, 'exit').then(() => true);
  child.kill('SIGTERM');
  const exited = await Promise.race([
    gracefulExit,
    new Promise<boolean>((resolveDelay) => setTimeout(() => resolveDelay(false), 3_000)),
  ]);
  if (!exited && child.exitCode === null && child.signalCode === null) {
    const forcedExit = once(child, 'exit');
    child.kill('SIGKILL');
    await Promise.race([
      forcedExit,
      new Promise((resolveDelay) => setTimeout(resolveDelay, 3_000)),
    ]);
  }
}