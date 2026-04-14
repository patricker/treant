import styles from './BarChart.module.css';

interface BarChartProps {
  items: Array<{
    label: string;
    value: number;
    secondary?: number;
  }>;
  maxValue?: number;
}

export default function BarChart({ items, maxValue }: BarChartProps) {
  const max = maxValue ?? Math.max(...items.map((d) => d.value), 1);

  return (
    <div className={styles.chart}>
      {items.map((item) => (
        <div key={item.label} className={styles.row}>
          <span className={styles.label}>{item.label}</span>
          <div className={styles.barTrack}>
            <div
              className={styles.barFill}
              style={{ width: `${(item.value / max) * 100}%` }}
            />
          </div>
          <div className={styles.values}>
            <span className={styles.primary}>{item.value.toLocaleString()}</span>
            {item.secondary != null && (
              <span className={styles.secondary}>{item.secondary.toFixed(2)}</span>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
