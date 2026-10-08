import { useState, useSyncExternalStore } from 'react';
import { AlertTriangle, X, Eye, Radio } from 'lucide-react';
import { useExecutionStore } from '@/store/execution-store';
import { useModelStore } from '@/store/model-store';
import { useSessionStore } from '@/store/session-store';
import { wsClient } from '@/client/websocket';

function useWsConnected(): boolean {
  return useSyncExternalStore(
    (cb) => wsClient.onStatus(cb),
    () => wsClient.connected,
  );
}

interface Props {
  onSubscribeSession: (sessionId: string) => void;
  onUnsubscribeSession: () => void;
}

export default function StatusBar({ onSubscribeSession, onUnsubscribeSession }: Props) {
  const running = useExecutionStore((s) => s.running);
  const paused = useExecutionStore((s) => s.paused);
  const issues = useExecutionStore((s) => s.issues);
  const setIssues = useExecutionStore((s) => s.setIssues);
  const checkIssues = useExecutionStore((s) => s.checkIssues);
  const setCheckIssues = useExecutionStore((s) => s.setCheckIssues);
  const fulfillment = useExecutionStore((s) => s.fulfillment);
  const hasModels = useModelStore((s) => s.models.length > 0);
  const connected = useWsConnected();

  const sessions = useSessionStore((s) => s.sessions);
  const subscribedSessionId = useSessionStore((s) => s.subscribedSessionId);
  const observing = useSessionStore((s) => s.observing);

  const [expandedPanel, setExpandedPanel] = useState<{
    source: 'issues' | 'check' | 'closed';
    issues: string[];
    checkIssues: string[];
  } | null>(null);
  const [showSessions, setShowSessions] = useState(false);
  const [connecting, setConnecting] = useState(false);

  const reconnect = async () => {
    setConnecting(true);
    try {
      await wsClient.connect();
      const response = await wsClient.send({ command: 'listSessions' });
      if (response.success) {
        useSessionStore.getState().setSessions(
          (response.sessions as Array<{ id: string; name: string }>) ?? [],
        );
      }
    } catch (error) {
      setIssues([error instanceof Error ? error.message : String(error)]);
    } finally {
      setConnecting(false);
    }
  };

  const vals = Object.values(fulfillment);
  const pct = vals.length === 0 ? 0 : vals.reduce((a, b) => a + b, 0) / vals.length;
  const hasIssues = issues.length > 0;
  const hasCheckIssues = checkIssues.length > 0;
  const checkOk = hasModels && !hasCheckIssues;
  const panelDataChanged = expandedPanel !== null
    && (expandedPanel.issues !== issues || expandedPanel.checkIssues !== checkIssues);
  const activeSource = panelDataChanged || expandedPanel === null
    ? hasIssues ? 'issues' : hasCheckIssues ? 'check' : null
    : expandedPanel.source === 'closed' ? null : expandedPanel.source;

  const visibleIssues = activeSource === 'issues' ? issues
    : activeSource === 'check' ? checkIssues
    : [];
  const visibleLabel = activeSource === 'issues' ? 'Execution Issues' : 'Model Check';
  const dismissVisible = () => {
    const nextIssues = activeSource === 'issues' ? [] : issues;
    const nextCheckIssues = activeSource === 'check' ? [] : checkIssues;
    if (activeSource === 'issues') setIssues(nextIssues);
    if (activeSource === 'check') setCheckIssues(nextCheckIssues);
    setExpandedPanel({ source: 'closed', issues: nextIssues, checkIssues: nextCheckIssues });
  };

  return (
    <div className="shrink-0">
      {visibleIssues.length > 0 && (
        <div className="border-t border-border bg-surface max-h-48 overflow-y-auto">
          <div className="flex items-center justify-between px-3 py-1.5 bg-danger/10 border-b border-border">
            <span className="text-xs font-semibold text-danger flex items-center gap-1.5">
              <AlertTriangle size={12} />
              {visibleLabel} ({visibleIssues.length})
            </span>
            <button
              onClick={dismissVisible}
              className="text-text-muted hover:text-text p-0.5 rounded transition-colors"
              title="Dismiss this execution or model-check message list."
            >
              <X size={12} />
            </button>
          </div>
          <ul className="px-3 py-1.5 space-y-0.5">
            {visibleIssues.map((issue, i) => (
              <li key={i} className="text-xs text-text-muted py-0.5 flex gap-2">
                <span className="text-danger shrink-0">&#x2022;</span>
                <span>{issue}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {showSessions && sessions.length > 0 && (
        <div className="border-t border-border bg-surface max-h-48 overflow-y-auto">
          <div className="flex items-center justify-between px-3 py-1.5 bg-primary/10 border-b border-border">
            <span className="text-xs font-semibold text-primary flex items-center gap-1.5">
              <Radio size={12} />
              Active Sessions ({sessions.length})
            </span>
            <button
              onClick={() => setShowSessions(false)}
              className="text-text-muted hover:text-text p-0.5 rounded transition-colors"
              title="Close the active-session list."
            >
              <X size={12} />
            </button>
          </div>
          <ul className="px-3 py-1.5 space-y-0.5">
            {sessions.map((s) => {
              const active = subscribedSessionId === s.id;
              return (
                <li
                  key={s.id}
                  title={active
                    ? `Stop observing ${s.name} and return to the local editor.`
                    : `Subscribe to ${s.name} to watch its live model execution.`}
                  onClick={() => {
                    if (active) { onUnsubscribeSession(); }
                    else { onSubscribeSession(s.id); }
                    setShowSessions(false);
                  }}
                  className={`flex items-center gap-1.5 text-xs py-1 px-1.5 rounded cursor-pointer transition-colors ${
                    active
                      ? 'bg-primary/15 text-primary'
                      : 'text-text hover:bg-surface-alt'
                  }`}
                >
                  <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${active ? 'bg-primary' : 'bg-success animate-pulse'}`} />
                  <span className="flex-1">{s.name}</span>
                  {active && <Eye size={10} className="shrink-0" />}
                </li>
              );
            })}
          </ul>
        </div>
      )}

      <div className="flex items-center h-7 bg-surface border-t border-border px-3 text-xs">
        <button
          type="button"
          className={`flex items-center gap-1.5 mr-3 ${!connected && !connecting ? 'cursor-pointer hover:text-text' : ''}`}
          onClick={reconnect}
          disabled={connected || connecting}
          title={connected
            ? 'Studio is connected to the GraphWalker execution service.'
            : connecting
              ? 'Connecting to the GraphWalker execution service.'
              : 'Connect to the GraphWalker execution service.'}
          aria-busy={connecting}
        >
          <span
            className={`w-2 h-2 rounded-full ${connected ? 'bg-success' : 'bg-danger'}`}
          />
          <span className="text-text-muted">
            {connected ? 'Connected' : connecting ? 'Connecting...' : 'Disconnected'}
          </span>
        </button>

        {hasModels && (
          <button
            className="flex items-center gap-1.5 mr-3"
            onClick={() => {
              if (!hasCheckIssues) return;
              setExpandedPanel({
                source: activeSource === 'check' ? 'closed' : 'check',
                issues,
                checkIssues,
              });
            }}
            title={checkOk
              ? 'The current models passed validation. Click to review model-check results when issues are present.'
              : `${checkIssues.length} model issue${checkIssues.length > 1 ? 's' : ''}. Click to view validation details.`}
          >
            <span
              className={`w-2 h-2 rounded-full ${checkOk ? 'bg-success' : 'bg-danger'}`}
            />
            <span className={checkOk ? 'text-text-muted' : 'text-danger'}>
              {checkOk ? 'Model OK' : `${checkIssues.length} model issue${checkIssues.length > 1 ? 's' : ''}`}
            </span>
          </button>
        )}

        {sessions.length > 0 && (
          <button
            className="flex items-center gap-1.5 mr-3"
            onClick={() => setShowSessions(!showSessions)}
            title={observing
              ? `Currently watching ${sessions.find((s) => s.id === subscribedSessionId)?.name ?? 'a session'}. Click to choose another active session.`
              : `${sessions.length} active session${sessions.length > 1 ? 's' : ''} available. Click to view and observe one.`}
          >
            <Radio size={12} className={observing ? 'text-primary' : 'text-text-muted'} />
            <span className={observing ? 'text-primary' : 'text-text-muted'}>
              {observing
                ? `Watching: ${sessions.find((s) => s.id === subscribedSessionId)?.name ?? 'session'}`
                : `Sessions (${sessions.length})`}
            </span>
          </button>
        )}

        {(running || paused) && (
          <div className="flex items-center gap-2 flex-1">
            <div
              className="w-48 h-1.5 bg-surface-alt rounded-full overflow-hidden"
              title={`Walk progress: ${(pct * 100).toFixed(0)}% of the configured stop condition.`}
            >
              <div
                className={`h-full rounded-full transition-all duration-300 ${
                  hasIssues ? 'bg-danger' : 'bg-success'
                }`}
                style={{ width: `${Math.min(pct * 100, 100)}%` }}
              />
            </div>
            <span className="text-text-muted">{(pct * 100).toFixed(0)}%</span>
            {paused && <span className="text-warning">Paused</span>}
          </div>
        )}

        {hasIssues && (
          <button
            onClick={() => {
              setExpandedPanel({
                source: activeSource === 'issues' ? 'closed' : 'issues',
                issues,
                checkIssues,
              });
            }}
            className="text-danger ml-auto flex items-center gap-1.5 hover:text-danger/80 transition-colors"
            title="View execution errors reported during the current walk."
          >
            <AlertTriangle size={12} />
            {issues.length} issue{issues.length > 1 ? 's' : ''}
          </button>
        )}

        {!running && !paused && !hasIssues && (
          <span className="text-text-muted ml-auto">Ready</span>
        )}
      </div>
    </div>
  );
}
