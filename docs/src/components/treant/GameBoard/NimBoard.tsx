import styles from './NimBoard.module.css';

interface NimBoardProps {
  stones: number;
  currentPlayer: string;
  onMove?: (move: 'Take1' | 'Take2') => void;
  disabled?: boolean;
}

export default function NimBoard({ stones, currentPlayer, onMove, disabled }: NimBoardProps) {
  const stoneElements = Array.from({ length: stones }, (_, i) => (
    <div key={i} className={styles.stone} />
  ));

  return (
    <div className={styles.board}>
      <div className={styles.player}>
        {currentPlayer}&apos;s turn
      </div>
      <div className={styles.stones}>
        {stoneElements.length > 0 ? stoneElements : (
          <span className={styles.empty}>No stones remaining</span>
        )}
      </div>
      {onMove && (
        <div className={styles.actions}>
          <button
            className="button button--sm button--outline button--primary"
            onClick={() => onMove('Take1')}
            disabled={disabled || stones < 1}
          >
            Take 1
          </button>
          <button
            className="button button--sm button--outline button--primary"
            onClick={() => onMove('Take2')}
            disabled={disabled || stones < 2}
          >
            Take 2
          </button>
        </div>
      )}
    </div>
  );
}
