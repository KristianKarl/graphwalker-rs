import {
  FilePlus,
  FolderOpen,
  Save,
  Undo2,
  Redo2,
  Play,
  Pause,
  SkipForward,
  Square,
  PanelRight,
  Sun,
  Moon,
} from 'lucide-react';
import { useEditorStore } from '@/store/editor-store';
import { useModelStore } from '@/store/model-store';

interface Props {
  onNewModel: () => void;
  onOpenFile: () => void;
  onSaveFile: () => void;
  onPlay: () => void;
  onPause: () => void;
  onStep: () => void;
  onStop: () => void;
  running: boolean;
  paused: boolean;
  observing?: boolean;
}

function IconButton({
  icon: Icon,
  title,
  onClick,
  disabled,
  active,
}: {
  icon: React.ElementType;
  title: string;
  onClick: () => void;
  disabled?: boolean;
  active?: boolean;
}) {
  return (
    <button
      title={title}
      aria-label={title}
      onClick={onClick}
      disabled={disabled}
      className={`
        flex items-center justify-center w-10 h-10 rounded-lg transition-colors
        ${active ? 'bg-primary/20 text-primary' : 'text-text-muted hover:text-text hover:bg-surface-alt'}
        ${disabled ? 'opacity-30 cursor-not-allowed' : 'cursor-pointer'}
      `}
    >
      <Icon size={18} />
    </button>
  );
}

export default function Sidebar(props: Props) {
  const { showProperties, toggleProperties, theme, setTheme } = useEditorStore();
  const canUndo = useModelStore((s) => s._past.length > 0);
  const canRedo = useModelStore((s) => s._future.length > 0);
  const undo = useModelStore((s) => s.undo);
  const redo = useModelStore((s) => s.redo);

  return (
    <div className="flex flex-col items-center w-12 bg-surface border-r border-border py-3 gap-1 shrink-0">
      <div className="flex flex-col gap-1 pb-3 border-b border-border mb-1">
        <IconButton icon={FilePlus} title="New model: create a blank model in a new tab." onClick={props.onNewModel} />
        <IconButton icon={FolderOpen} title="Open model: load a JSON or GraphML model file." onClick={props.onOpenFile} />
        <IconButton icon={Save} title="Save models: download the open models as a JSON test file." onClick={props.onSaveFile} />
      </div>

      <div className="flex flex-col gap-1 pb-3 border-b border-border mb-1">
        <IconButton icon={Undo2} title="Undo the last model edit (Ctrl+Z)." onClick={undo} disabled={!canUndo} />
        <IconButton icon={Redo2} title="Redo the most recently undone model edit (Ctrl+Shift+Z)." onClick={redo} disabled={!canRedo} />
      </div>

      <div className="flex flex-col gap-1 pb-3 border-b border-border mb-1">
        {props.running && !props.paused ? (
          <IconButton icon={Pause} title="Pause the current walk. Play resumes from this point." onClick={props.onPause} />
        ) : (
          <IconButton icon={Play} title="Start or resume walking the model using its selected generator." onClick={props.onPlay} />
        )}
        <IconButton
          icon={SkipForward}
          title="Step: advance the walk by one element. In observed sessions, request one remote step."
          onClick={props.onStep}
          disabled={props.running && !props.paused && !props.observing}
        />
        <IconButton
          icon={Square}
          title="Stop the current walk or leave the observed session and clear its progress."
          onClick={props.onStop}
          disabled={!props.running && !props.paused}
        />
      </div>

      <div className="flex flex-col gap-1">
        <IconButton
          icon={PanelRight}
          title="Show or hide the properties panel for the model and selected element."
          onClick={toggleProperties}
          active={showProperties}
        />
        <IconButton
          icon={theme === 'dark' ? Sun : Moon}
          title={`Switch to the ${theme === 'dark' ? 'light' : 'dark'} color theme.`}
          onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
        />
      </div>
    </div>
  );
}
