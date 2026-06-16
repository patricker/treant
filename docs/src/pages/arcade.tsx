import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';

export default function ArcadePage(): JSX.Element {
  return (
    <Layout title="Arcade" description="Pass-and-play family games powered by treant MCTS">
      <BrowserOnly fallback={<div style={{ padding: 40, textAlign: 'center' }}>Loading arcade…</div>}>
        {() => {
          const { WasmProvider, useWasm } = require('@site/src/components/treant/WasmProvider');
          const ArcadeShell = require('@site/src/components/arcade/ArcadeShell').default;
          function Inner() {
            const { wasm, ready, error } = useWasm();
            if (error) return <div style={{ padding: 40, textAlign: 'center' }}>Failed to load: {String(error)}</div>;
            if (!ready) return <div style={{ padding: 40, textAlign: 'center' }}>Loading arcade…</div>;
            return <ArcadeShell wasm={wasm} />;
          }
          return (
            <WasmProvider>
              <Inner />
            </WasmProvider>
          );
        }}
      </BrowserOnly>
    </Layout>
  );
}
