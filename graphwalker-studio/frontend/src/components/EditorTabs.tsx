import { X, Plus } from 'lucide-react';
import type { GWModel } from '@/store/types';
import { useExecutionStore } from '@/store/execution-store';

interface Props {
  models: GWModel[];
  selectedIndex: number;
  onSelect: (index: number) => void;
  onClose: (index: number) => void;
  onAdd: () => void;
}

export default function EditorTabs({ models, selectedIndex, onSelect, onClose, onAdd }: Props) {
  const visited = useExecutionStore((s) => s.visited);
  const running = useExecutionStore((s) => s.running);
  const paused = useExecutionStore((s) => s.paused);

  if (models.length === 0) return null;

  return (
    <div
      role="tablist"
      aria-label="Models"
      className="flex items-center bg-surface border-b border-border h-9 shrink-0 overflow-x-auto"
    >
      {models.map((m, i) => {
        const hasVisited = (running || paused) && visited[m.id] && Object.keys(visited[m.id]).length > 0;
        return (
          <div
            key={m.id}
            className={`flex items-center h-full border-r border-border ${
              i === selectedIndex ? 'bg-background text-text' : 'text-text-muted'
            }`}
          >
            <button
              id={`model-tab-${m.id}`}
              type="button"
              role="tab"
              aria-selected={i === selectedIndex}
              aria-controls={`model-panel-${m.id}`}
              onClick={() => onSelect(i)}
              title={`${m.name}${i === selectedIndex ? ' (active model)' : '. Click to edit this model'}${hasVisited ? '. This model has execution progress.' : ''}`}
              className={`flex items-center gap-2 px-3 h-full text-sm transition-colors select-none whitespace-nowrap ${
                i === selectedIndex
                  ? 'border-b-2 border-b-primary'
                  : 'hover:text-text hover:bg-surface-alt'
              }`}
            >
              {hasVisited && (
                <span className="w-1.5 h-1.5 rounded-full bg-success shrink-0" />
              )}
              <span>{m.name}</span>
            </button>
            <button
              type="button"
              aria-label={`Close ${m.name}`}
              title={`Close the ${m.name} tab. This removes it from the current workspace.`}
              onClick={(e) => { e.stopPropagation(); onClose(i); }}
              className="hover:text-danger rounded p-0.5 transition-colors"
            >
              <X size={12} />
            </button>
          </div>
        );
      })}
      <button
        type="button"
        aria-label="New model"
        onClick={onAdd}
        className="flex items-center justify-center w-9 h-full text-text-muted hover:text-text hover:bg-surface-alt transition-colors"
        title="New model: create a blank model in a new tab."
      >
        <Plus size={14} />
      </button>
    </div>
  );
}
