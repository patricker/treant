import styles from './ParameterControls.module.css';

interface ParamConfig {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
}

interface ParameterControlsProps {
  params: Record<string, ParamConfig>;
  onChange: (key: string, value: number) => void;
}

export default function ParameterControls({ params, onChange }: ParameterControlsProps) {
  return (
    <div className={styles.controls}>
      {Object.entries(params).map(([key, config]) => (
        <div key={key} className={styles.row}>
          <label className={styles.label} htmlFor={`param-${key}`}>
            {config.label}
          </label>
          <input
            id={`param-${key}`}
            className={styles.slider}
            type="range"
            min={config.min}
            max={config.max}
            step={config.step}
            value={config.value}
            onChange={(e) => onChange(key, parseFloat(e.target.value))}
          />
          <span className={styles.value}>{config.value.toFixed(2)}</span>
        </div>
      ))}
    </div>
  );
}
