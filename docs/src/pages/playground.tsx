import { useState } from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';
import styles from './playground.module.css';

const tabs = [
  { id: 'counting', label: 'Counting Game' },
  { id: 'nim', label: 'Nim' },
  { id: 'dice', label: 'Dice Game' },
  { id: 'compare', label: 'Compare Policies' },
] as const;

type TabId = (typeof tabs)[number]['id'];

function DemoLoader({ tab }: { tab: TabId }) {
  switch (tab) {
    case 'counting': {
      const StepThroughDemo =
        require('@site/src/components/demos/StepThroughDemo').default;
      return <StepThroughDemo />;
    }
    case 'nim': {
      const NimSolverDemo =
        require('@site/src/components/demos/NimSolverDemo').default;
      return <NimSolverDemo />;
    }
    case 'dice': {
      const ChanceNodesDemo =
        require('@site/src/components/demos/ChanceNodesDemo').default;
      return <ChanceNodesDemo />;
    }
    case 'compare': {
      const UCTvsPUCTDemo =
        require('@site/src/components/demos/UCTvsPUCTDemo').default;
      return <UCTvsPUCTDemo />;
    }
  }
}

export default function Playground(): JSX.Element {
  const [activeTab, setActiveTab] = useState<TabId>('counting');

  return (
    <Layout
      title="Playground"
      description="Interactive MCTS demos powered by WASM"
    >
      <main className={styles.playground}>
        <div className={styles.header}>
          <h1>Playground</h1>
          <p>Interactive demos running the actual MCTS library via WebAssembly.</p>
        </div>

        <div className={styles.tabBar}>
          {tabs.map((tab) => (
            <button
              key={tab.id}
              className={`${styles.tab} ${activeTab === tab.id ? styles.tabActive : ''}`}
              onClick={() => setActiveTab(tab.id)}
              type="button"
            >
              {tab.label}
            </button>
          ))}
        </div>

        <div className={styles.tabContent}>
          <BrowserOnly fallback={<div className={styles.loading}>Loading demo...</div>}>
            {() => <DemoLoader tab={activeTab} />}
          </BrowserOnly>
        </div>
      </main>
    </Layout>
  );
}
