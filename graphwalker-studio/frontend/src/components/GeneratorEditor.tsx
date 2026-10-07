import { useId, useState } from 'react';
import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react';
import type { GWModel } from '@/store/types';

const generators = [
  ['random', 'Random'], ['quick_random', 'Quick random'],
  ['weighted_random', 'Weighted random'], ['a_star', 'A*'],
  ['shortest_all_paths', 'Shortest all paths'], ['predefined_path', 'Predefined path'],
  ['new_york_street_sweeper', 'New York street sweeper'],
] as const;
const conditions = [
  ['edge_coverage', 'Edge coverage', '%'], ['vertex_coverage', 'Vertex coverage', '%'],
  ['requirement_coverage', 'Requirement coverage', '%'],
  ['dependency_edge_coverage', 'Dependency edge coverage', '%'],
  ['length', 'Length', 'element visits'], ['time_duration', 'Duration', 'seconds'],
  ['reached_vertex', 'Reached vertex', 'name'], ['reached_edge', 'Reached edge', 'name'],
  ['reached_shared_state', 'Reached shared state', 'name'],
  ['predefined_path', 'Predefined path', ''], ['never', 'Never', ''],
] as const;
type Stage = { generator: string; condition: string; argument: string };
const control = 'w-full min-w-0 bg-surface-alt border border-border rounded-md px-2 py-1.5 text-sm text-text focus:outline-none focus:border-primary';
const iconButton = 'p-1.5 rounded hover:bg-surface-alt text-text-muted hover:text-text disabled:opacity-30 disabled:cursor-not-allowed';

function canonical(name: string, names: readonly string[]) {
  return names.find((candidate) => candidate.replaceAll('_', '') === name.toLowerCase().replaceAll('_', ''));
}

function parseStages(expression: string): Stage[] | null {
  const pattern = /\s*([a-z_][a-z_0-9]*)\s*\(\s*(?:([a-z_][a-z_0-9]*)\s*(?:\(\s*([^()]*)\s*\))?)?\s*\)/iy;
  const stages: Stage[] = [];
  while (pattern.lastIndex < expression.trimEnd().length) {
    const match = pattern.exec(expression);
    if (!match) return null;
    const rawGenerator = match[1].toLowerCase().replaceAll('_', '').replace(/path$/, '');
    const generator = canonical(match[1], generators.map(([name]) => name))
      ?? ({ random: 'random', quickrandom: 'quick_random', weightedrandom: 'weighted_random', astar: 'a_star' } as Record<string, string>)[rawGenerator];
    if (!generator) return null;
    if (generator === 'new_york_street_sweeper') {
      if (match[2]) return null;
      stages.push({ generator, condition: '', argument: '' });
      continue;
    }
    if (!match[2]) return null;
    const condition = canonical(match[2], conditions.map(([name]) => name));
    if (!condition) return null;
    const unit = conditions.find(([name]) => name === condition)![2];
    const argument = (match[3] ?? '').trim();
    if (unit ? match[3] === undefined : match[3] !== undefined) return null;
    if (unit && unit !== 'name' && !/^\d*$/.test(argument)) return null;
    stages.push({ generator, condition, argument });
  }
  return stages.length ? stages : null;
}

function serialize(stages: Stage[]) {
  return stages.map(({ generator, condition, argument }) =>
    `${generator}(${condition}${conditions.find(([name]) => name === condition)?.[2] ? `(${argument})` : ''})`,
  ).join(' ');
}

export default function GeneratorEditor({ model, onChange }: { model: GWModel; onChange: (value: string) => void }) {
  const id = useId();
  const [textMode, setTextMode] = useState(false);
  const stages = parseStages(model.generator);
  const showText = textMode || !stages;
  const update = (index: number, changes: Partial<Stage>) => {
    if (stages) onChange(serialize(stages.map((stage, position) => position === index ? { ...stage, ...changes } : stage)));
  };
  const move = (index: number, offset: number) => {
    if (!stages) return;
    const reordered = [...stages];
    [reordered[index], reordered[index + offset]] = [reordered[index + offset], reordered[index]];
    onChange(serialize(reordered));
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs text-text-muted">Generator</span>
        <div role="group" aria-label="Generator editor mode" className="flex border border-border rounded-md overflow-hidden text-xs">
          <button type="button" aria-pressed={!showText} disabled={!stages} onClick={() => setTextMode(false)}
            title={stages ? 'Edit generator stages' : 'This expression requires text mode'}
            className={`px-2 py-1 disabled:opacity-40 ${!showText ? 'bg-primary text-white' : 'text-text-muted'}`}>Builder</button>
          <button type="button" aria-pressed={showText} onClick={() => setTextMode(true)}
            className={`px-2 py-1 ${showText ? 'bg-primary text-white' : 'text-text-muted'}`}>Text</button>
        </div>
      </div>
      {showText ? (
        <textarea aria-label="Generator" value={model.generator} onChange={(event) => onChange(event.target.value)}
          className={`${control} font-mono resize-y min-h-24`} rows={4} spellCheck={false} />
      ) : stages.map((stage, index) => {
        const unit = conditions.find(([name]) => name === stage.condition)?.[2];
        const names = stage.condition === 'reached_vertex' ? model.vertices.map((vertex) => vertex.name)
          : stage.condition === 'reached_edge' ? model.edges.map((edge) => edge.name)
          : model.vertices.flatMap((vertex) => vertex.sharedState ? [vertex.sharedState] : []);
        const invalid = unit && (unit === 'name' ? !stage.argument || /[()]/.test(stage.argument)
          : !/^\d+$/.test(stage.argument) || (unit === '%' && Number(stage.argument) > 100)
            || BigInt(stage.argument || '0') > (unit === '%' ? 4294967295n : 18446744073709551615n));
        return (
          <fieldset key={index} aria-label={`Generator stage ${index + 1}`} className="min-w-0 border-t border-border pt-2 space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-xs text-text-muted">Stage {index + 1}</span>
              <div className="flex">
                <button type="button" className={iconButton} title={`Move stage ${index + 1} up`} aria-label={`Move stage ${index + 1} up`} disabled={index === 0} onClick={() => move(index, -1)}><ArrowUp size={14} /></button>
                <button type="button" className={iconButton} title={`Move stage ${index + 1} down`} aria-label={`Move stage ${index + 1} down`} disabled={index === stages.length - 1} onClick={() => move(index, 1)}><ArrowDown size={14} /></button>
                <button type="button" className={iconButton} title={`Remove stage ${index + 1}`} aria-label={`Remove stage ${index + 1}`} disabled={stages.length === 1} onClick={() => onChange(serialize(stages.filter((_, position) => position !== index)))}><Trash2 size={14} /></button>
              </div>
            </div>
            <label className="block text-xs text-text-muted space-y-1">
              <span>Generator</span>
              <select aria-label="Generator" className={control} value={stage.generator} onChange={(event) => {
                const generator = event.target.value;
                const needsTarget = generator === 'a_star' && !stage.condition.startsWith('reached_');
                update(index, generator === 'new_york_street_sweeper' ? { generator, condition: '', argument: '' }
                  : generator === 'predefined_path' ? { generator, condition: 'predefined_path', argument: '' }
                  : needsTarget ? { generator, condition: 'reached_vertex', argument: model.vertices[0]?.name ?? '' }
                  : { generator, condition: stage.condition || 'edge_coverage', argument: stage.condition ? stage.argument : '100' });
              }}>
                {generators.map(([name, label]) => <option key={name} value={name}>{label}</option>)}
              </select>
            </label>
            {stage.generator !== 'new_york_street_sweeper' && <>
              <label className="block text-xs text-text-muted space-y-1">
                <span>Stop when</span>
                <select aria-label="Stop when" className={control} value={stage.condition} onChange={(event) => {
                  const condition = event.target.value;
                  const nextUnit = conditions.find(([name]) => name === condition)![2];
                  update(index, { condition, argument: nextUnit === 'name'
                    ? (condition === 'reached_vertex' ? model.vertices[0]?.name : condition === 'reached_edge' ? model.edges[0]?.name : model.vertices.find((vertex) => vertex.sharedState)?.sharedState) ?? ''
                    : nextUnit === '%' || condition === 'length' ? '100' : nextUnit === 'seconds' ? '60' : '' });
                }}>
                  {conditions.map(([name, label]) => <option key={name} value={name}>{label}</option>)}
                </select>
              </label>
              {unit && <div>
                <label className="block text-xs text-text-muted space-y-1">
                  <span>{unit === 'name' ? 'Target name' : `Value (${unit})`}</span>
                  <input className={control} type={unit === 'name' ? 'text' : 'number'} min={unit === 'name' ? undefined : 0}
                    max={unit === '%' ? 100 : undefined} step={unit === 'name' ? undefined : 1}
                    list={unit === 'name' ? `${id}-${index}-names` : undefined} value={stage.argument}
                    aria-invalid={Boolean(invalid)} onChange={(event) => {
                      if (unit === 'name' || /^\d*$/.test(event.target.value)) update(index, { argument: event.target.value });
                    }} />
                </label>
                {unit === 'name' && <datalist id={`${id}-${index}-names`}>{[...new Set(names)].filter(Boolean).map((name) => <option key={name} value={name} />)}</datalist>}
                {invalid && <p className="text-xs text-warning mt-1" role="alert">{unit === 'name' ? 'Enter a target name without parentheses.' : unit === '%' ? 'Enter a whole number from 0 to 100.' : 'Enter a non-negative whole number within the supported range.'}</p>}
              </div>}
            </>}
          </fieldset>
        );
      })}
      {!showText && <>
        <button type="button" onClick={() => onChange(serialize([...stages!, { generator: 'random', condition: 'edge_coverage', argument: '100' }]))}
          className="flex items-center gap-1.5 text-xs text-primary py-1"><Plus size={14} />Add generator</button>
        <label className="block text-xs text-text-muted space-y-1">
          <span>Expression</span>
          <textarea aria-label="Generator expression" readOnly value={model.generator} rows={3} className={`${control} font-mono resize-y`} />
        </label>
      </>}
    </div>
  );
}