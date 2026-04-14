import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import styles from './index.module.css';

const features = [
  {
    title: 'Learn MCTS',
    description:
      'Progressive tutorials from theory through AlphaZero-style search.',
    link: '/docs/tutorials/01-what-is-mcts',
    linkText: 'Start tutorials',
  },
  {
    title: 'Interactive Demos',
    description:
      'Every concept backed by a live interactive demo you can manipulate.',
    link: '/playground',
    linkText: 'Open playground',
  },
  {
    title: 'Production Code',
    description:
      'All examples compiled and tested from the actual Rust source.',
    link: '/docs/intro',
    linkText: 'Browse docs',
  },
];

const capabilities = [
  'Lock-free parallel search',
  'UCT + PUCT tree policies',
  'MCTS-Solver (game-theoretic proving)',
  'Score-Bounded MCTS',
  'Open/closed-loop chance nodes',
  'Batched neural network evaluation',
  'Tree reuse across turns',
  'Progressive widening',
];

export default function Home(): JSX.Element {
  return (
    <Layout
      title="Treant"
      description="High-performance, lock-free Monte Carlo Tree Search for Rust"
    >
      <main>
        {/* Hero */}
        <section className={styles.hero}>
          <div className="container">
            <h1 className={styles.heroTitle}>Treant</h1>
            <p className={styles.heroSubtitle}>
              High-performance, lock-free Monte Carlo Tree Search for Rust
            </p>
            <div className={styles.heroCtas}>
              <Link
                className={styles.ctaPrimary}
                to="/docs/tutorials/01-what-is-mcts"
              >
                Start Learning
              </Link>
              <Link className={styles.ctaSecondary} to="/playground">
                Playground
              </Link>
            </div>
          </div>
        </section>

        {/* Features */}
        <section className={styles.features}>
          <div className="container">
            <div className={styles.featureCards}>
              {features.map((f) => (
                <div key={f.title} className={styles.featureCard}>
                  <h3>{f.title}</h3>
                  <p>{f.description}</p>
                  <Link className={styles.featureLink} to={f.link}>
                    {f.linkText} &rarr;
                  </Link>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Capabilities */}
        <section className={styles.capabilities}>
          <div className="container">
            <h2>Capabilities</h2>
            <div className={styles.capGrid}>
              {capabilities.map((cap) => (
                <div key={cap} className={styles.capItem}>
                  <span className={styles.capBullet} />
                  {cap}
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Code preview */}
        <section className={styles.codePreview}>
          <div className="container">
            <h2>Minimal interface</h2>
            <p>
              Implement three methods and the library handles the rest.
            </p>
            <pre className={styles.codeBlock}>
              <code>
                <span className={styles.keyword}>pub trait</span>{' '}
                <span className={styles.type}>GameState</span>
                {': '}
                <span className={styles.type}>Clone</span>
                {' {\n'}
                {'    '}
                <span className={styles.keyword}>type</span> Move: Sync + Send
                + Clone;{'\n'}
                {'    '}
                <span className={styles.keyword}>type</span> Player: Sync;
                {'\n'}
                {'    '}
                <span className={styles.keyword}>type</span> MoveList:
                IntoIterator{'<'}Item = Self::Move{'>'};{'\n'}
                {'\n'}
                {'    '}
                <span className={styles.comment}>
                  {'/// The player whose turn it is.'}
                </span>
                {'\n'}
                {'    '}
                <span className={styles.keyword}>fn</span>{' '}
                current_player({'&'}
                <span className={styles.keyword}>self</span>) {'-> '}
                Self::Player;{'\n'}
                {'\n'}
                {'    '}
                <span className={styles.comment}>
                  {'/// Legal moves from this state. Empty means terminal.'}
                </span>
                {'\n'}
                {'    '}
                <span className={styles.keyword}>fn</span>{' '}
                available_moves({'&'}
                <span className={styles.keyword}>self</span>) {'-> '}
                Self::MoveList;{'\n'}
                {'\n'}
                {'    '}
                <span className={styles.comment}>
                  {'/// Apply a move, mutating the state in place.'}
                </span>
                {'\n'}
                {'    '}
                <span className={styles.keyword}>fn</span> make_move({'&'}
                <span className={styles.keyword}>mut</span>{' '}
                <span className={styles.keyword}>self</span>, mov: {'&'}
                Self::Move);{'\n'}
                {'}'}
              </code>
            </pre>
          </div>
        </section>
      </main>
    </Layout>
  );
}
