import { useId } from 'react';
import { useModelStore } from '@/store/model-store';
import { useExecutionStore } from '@/store/execution-store';
import { useEditorStore } from '@/store/editor-store';
import type { GWModel } from '@/store/types';
import GeneratorEditor from './GeneratorEditor';

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section aria-label={title} className="border-b border-border">
      <h2 className="px-4 py-2 text-xs font-semibold text-text-muted uppercase tracking-wider bg-surface-alt">
        {title}
      </h2>
      <div className="p-4 space-y-3">{children}</div>
    </section>
  );
}

function Field({
  label,
  value,
  onChange,
  help,
  disabled,
  multiline,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  help: string;
  disabled?: boolean;
  multiline?: boolean;
}) {
  const id = useId();
  const cls = `
    w-full bg-surface-alt border border-border rounded-md px-3 py-1.5
    text-sm text-text focus:outline-none focus:border-primary
    disabled:opacity-40 disabled:cursor-not-allowed
    transition-colors
  `;
  return (
    <div>
      <label htmlFor={id} className="block text-xs text-text-muted mb-1">{label}</label>
      {multiline ? (
        <textarea
          id={id}
          className={`${cls} resize-y min-h-[60px]`}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
          title={help}
          rows={3}
        />
      ) : (
        <input
          id={id}
          className={cls}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
          title={help}
        />
      )}
    </div>
  );
}

export default function PropertiesPanel() {
  const models = useModelStore((s) => s.models);
  const selectedModelIndex = useModelStore((s) => s.selectedModelIndex);
  const selectedElementId = useModelStore((s) => s.selectedElementId);
  const updateModel = useModelStore((s) => s.updateModel);
  const updateVertex = useModelStore((s) => s.updateVertex);
  const updateEdge = useModelStore((s) => s.updateEdge);
  const setStartElement = useModelStore((s) => s.setStartElement);
  const delay = useExecutionStore((s) => s.delay);
  const setDelay = useExecutionStore((s) => s.setDelay);
  const seed = useEditorStore((s) => s.seed);
  const setSeed = useEditorStore((s) => s.setSeed);
  const autoSeed = useEditorStore((s) => s.autoSeed);
  const setAutoSeed = useEditorStore((s) => s.setAutoSeed);
  const globalData = useEditorStore((s) => s.globalData);
  const setGlobalData = useEditorStore((s) => s.setGlobalData);

  const model: GWModel | undefined = models[selectedModelIndex];
  if (!model) {
    return (
      <div className="w-80 bg-surface border-l border-border shrink-0 flex items-center justify-center text-text-muted text-sm">
        No model selected
      </div>
    );
  }

  const vertex = model.vertices.find((v) => v.id === selectedElementId);
  const edge = model.edges.find((e) => e.id === selectedElementId);
  const noSelection = !vertex && !edge;

  return (
    <div className="w-80 bg-surface border-l border-border shrink-0 overflow-y-auto">
      <Section title="Global">
        <div>
          <div className="flex items-center justify-between mb-1">
            <label htmlFor="studio-seed" className="text-xs text-text-muted">Seed</label>
            <label className="flex items-center gap-1.5 text-xs text-text-muted cursor-pointer select-none">
              <input
                type="checkbox"
                checked={autoSeed}
                onChange={(e) => setAutoSeed(e.target.checked)}
                title="Generate a seed automatically for each new walk. Turn this off to enter a repeatable seed."
                className="accent-primary"
              />
              Auto
            </label>
          </div>
          <input
            id="studio-seed"
            className={`
              w-full bg-surface-alt border rounded-md px-3 py-1.5
              text-sm text-text focus:outline-none focus:border-primary
              disabled:opacity-40 disabled:cursor-not-allowed
              transition-colors
              ${!autoSeed && !seed.trim() ? 'border-warning' : 'border-border'}
            `}
            value={seed}
            onChange={(e) => setSeed(e.target.value)}
            disabled={autoSeed}
            title="Seed used by the generator to make a walk repeatable. Use the same seed and model to reproduce a run."
            placeholder={autoSeed ? '' : 'Enter a seed number'}
          />
          {!autoSeed && !seed.trim() && (
            <p className="text-xs text-warning mt-1">Enter a seed or enable auto</p>
          )}
        </div>
        <Field
          label="Global data"
          value={globalData}
          onChange={setGlobalData}
          help="Initial global data supplied to the model when a walk starts."
          multiline
        />
      </Section>

      <Section title="Execution">
        <GeneratorEditor
          key={model.id}
          model={model}
          onChange={(value) => updateModel(selectedModelIndex, { generator: value })}
        />
        <div>
          <label htmlFor="studio-step-delay" className="block text-xs text-text-muted mb-1">
            Step delay: {delay}ms
          </label>
          <input
            id="studio-step-delay"
            type="range"
            min={0}
            max={500}
            step={10}
            aria-label="Step delay"
            value={delay}
            onChange={(e) => setDelay(Number(e.target.value))}
            title="Wait this many milliseconds between walk steps. Set to zero to run without a delay."
            className="w-full accent-primary"
          />
        </div>
      </Section>

      <Section title="Model">
        <Field
          label="Name"
          value={model.name}
          onChange={(v) => updateModel(selectedModelIndex, { name: v })}
          help="Display name for this model and its editor tab."
        />
        <Field
          label="Actions"
          value={(model.actions ?? []).filter(Boolean).join('\n')}
          onChange={(v) => {
            const actions = v.split('\n').map((s) => s.trim()).filter(Boolean);
            updateModel(selectedModelIndex, { actions });
          }}
          help="Actions associated with this model. Enter one action per line."
          multiline
        />
      </Section>

      <Section title="Element">
        <Field
          label="Name"
          value={vertex?.name ?? edge?.name ?? ''}
          onChange={(v) => {
            if (vertex) updateVertex(selectedModelIndex, vertex.id, { name: v });
            if (edge) updateEdge(selectedModelIndex, edge.id, { name: v });
          }}
          help="Label shown for the selected vertex or edge in the graph."
          disabled={noSelection}
        />
        {(vertex || noSelection) && (
          <Field
            label="Shared State"
            value={vertex?.sharedState ?? ''}
            onChange={(v) => {
              if (vertex) updateVertex(selectedModelIndex, vertex.id, { sharedState: v || undefined });
            }}
            help="Shared-state identifier for this vertex. Vertices in different models with the same identifier can share execution state."
            disabled={noSelection}
          />
        )}
        {(edge || noSelection) && (
          <>
            <Field
              label="Guard"
              value={edge?.guard ?? ''}
              onChange={(v) => {
                if (edge) updateEdge(selectedModelIndex, edge.id, { guard: v || undefined });
              }}
              help="Condition that must be true for this edge to be eligible during a walk."
              disabled={noSelection}
            />
            <Field
              label="Weight"
              value={edge?.weight?.toString() ?? ''}
              onChange={(v) => {
                if (edge) updateEdge(selectedModelIndex, edge.id, { weight: v ? Number(v) : undefined });
              }}
              help="Relative probability weight used by generators that choose between eligible edges."
              disabled={noSelection}
            />
          </>
        )}
        <Field
          label="Actions"
          value={
            vertex ? (vertex.actions ?? []).filter(Boolean).join('\n')
            : edge ? (edge.actions ?? []).filter(Boolean).join('\n')
            : (model.actions ?? []).filter(Boolean).join('\n')
          }
          onChange={(v) => {
            const actions = v.split('\n').map((s) => s.trim()).filter(Boolean);
            if (vertex) updateVertex(selectedModelIndex, vertex.id, { actions });
            else if (edge) updateEdge(selectedModelIndex, edge.id, { actions });
            else updateModel(selectedModelIndex, { actions });
          }}
          help="Actions executed for this model, vertex, or edge. Enter one action per line."
          multiline
        />
        <Field
          label="Requirements"
          value={vertex?.requirements?.join('\n') ?? edge?.requirements?.join('\n') ?? ''}
          onChange={(v) => {
            const reqs = v.split('\n').map((s) => s.trim()).filter(Boolean);
            if (vertex) updateVertex(selectedModelIndex, vertex.id, { requirements: reqs });
            if (edge) updateEdge(selectedModelIndex, edge.id, { requirements: reqs });
          }}
          help="Requirements covered by this vertex or edge. Enter one requirement per line."
          disabled={noSelection}
          multiline
        />
        {(vertex || edge) && (
          <div className="flex items-center gap-2">
            <label className="text-xs text-text-muted">Start element</label>
            <button
              type="button"
              onClick={() => setStartElement(selectedModelIndex, (vertex?.id ?? edge?.id)!)}
              title={model.startElementId === (vertex?.id ?? edge?.id)
                ? 'This is the element where a new walk starts.'
                : 'Make this vertex or edge the starting point for new walks.'}
              className={`
                px-2 py-0.5 text-xs rounded-md border transition-colors
                ${model.startElementId === (vertex?.id ?? edge?.id)
                  ? 'bg-success/20 border-success text-success'
                  : 'border-border text-text-muted hover:border-border-hover'}
              `}
            >
              {model.startElementId === (vertex?.id ?? edge?.id) ? 'Start' : 'Set as start'}
            </button>
          </div>
        )}
      </Section>
    </div>
  );
}
