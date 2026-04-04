import styles from './StatsPanel.module.css';

interface StatsPanelProps {
  totalPlayouts: number;
  totalNodes: number;
  bestMove?: string;
  children?: Array<{
    mov: string;
    visits: number;
    avg_reward: number;
    prior?: number;
    proven?: string;
  }>;
}

export default function StatsPanel({
  totalPlayouts,
  totalNodes,
  bestMove,
  children,
}: StatsPanelProps) {
  const showPrior = children?.some((c) => c.prior != null) ?? false;
  const showStatus = children?.some((c) => c.proven != null) ?? false;

  return (
    <div className={styles.panel}>
      <div className={styles.summary}>
        <span className={styles.stat}>
          <span className={styles.label}>Playouts</span>
          <span className={styles.value}>{totalPlayouts.toLocaleString()}</span>
        </span>
        <span className={styles.stat}>
          <span className={styles.label}>Nodes</span>
          <span className={styles.value}>{totalNodes.toLocaleString()}</span>
        </span>
        {bestMove && (
          <span className={styles.stat}>
            <span className={styles.label}>Best</span>
            <span className={styles.value}>{bestMove}</span>
          </span>
        )}
      </div>

      {children && children.length > 0 && (
        <table className={styles.table}>
          <thead>
            <tr>
              <th>Move</th>
              <th>Visits</th>
              <th>Avg Reward</th>
              {showPrior && <th>Prior</th>}
              {showStatus && <th>Status</th>}
            </tr>
          </thead>
          <tbody>
            {children.map((child) => (
              <tr key={child.mov}>
                <td>{child.mov}</td>
                <td className={styles.mono}>{child.visits.toLocaleString()}</td>
                <td className={styles.mono}>{child.avg_reward.toFixed(2)}</td>
                {showPrior && (
                  <td className={styles.mono}>
                    {child.prior != null ? child.prior.toFixed(2) : '\u2014'}
                  </td>
                )}
                {showStatus && (
                  <td className={styles.statusCell} data-status={child.proven?.toLowerCase()}>
                    {child.proven ?? '\u2014'}
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
