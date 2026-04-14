import styles from './PlaybackControls.module.css';

interface PlaybackControlsProps {
  onStep: () => void;
  onRun: (n: number) => void;
  onReset: () => void;
  batchSizes?: number[];
}

function formatBatch(n: number): string {
  if (n >= 1000) return `Run ${(n / 1000)}K`;
  return `Run ${n}`;
}

export default function PlaybackControls({
  onStep,
  onRun,
  onReset,
  batchSizes = [10, 100, 1000],
}: PlaybackControlsProps) {
  return (
    <div className={styles.controls}>
      <button
        className="button button--sm button--outline button--primary"
        onClick={onStep}
      >
        Step
      </button>
      {batchSizes.map((n) => (
        <button
          key={n}
          className="button button--sm button--outline button--primary"
          onClick={() => onRun(n)}
        >
          {formatBatch(n)}
        </button>
      ))}
      <button
        className="button button--sm button--outline button--danger"
        onClick={onReset}
      >
        Reset
      </button>
    </div>
  );
}
