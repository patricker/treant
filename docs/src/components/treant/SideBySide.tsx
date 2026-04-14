import type { ReactNode } from 'react';
import styles from './SideBySide.module.css';

interface SideBySideProps {
  left: ReactNode;
  right: ReactNode;
  leftLabel?: string;
  rightLabel?: string;
}

export default function SideBySide({ left, right, leftLabel, rightLabel }: SideBySideProps) {
  return (
    <div className={styles.container}>
      <div className={styles.panel}>
        {leftLabel && <div className={styles.label}>{leftLabel}</div>}
        <div className={styles.content}>{left}</div>
      </div>
      <div className={styles.panel}>
        {rightLabel && <div className={styles.label}>{rightLabel}</div>}
        <div className={styles.content}>{right}</div>
      </div>
    </div>
  );
}
