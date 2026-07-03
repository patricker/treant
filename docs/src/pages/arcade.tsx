import type { JSX } from 'react';
import type React from 'react';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';

// The arcade is always the dark CRT skin, but this page's loading fallback
// renders before the skin mounts — on the default light site theme that meant
// a white flash on first load. Paint the placeholder dark from the first frame.
const loadingBox: React.CSSProperties = {
  minHeight: '70vh',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  background: '#05060a',
  color: '#8ce8a9',
  fontFamily: 'monospace',
  fontSize: '1rem',
};

export default function ArcadePage(): JSX.Element {
  return (
    <Layout title="Arcade" description="Pass-and-play family games powered by treant MCTS">
      <BrowserOnly fallback={<div style={loadingBox}>Loading arcade…</div>}>
        {() => {
          const { WasmProvider, useWasm } = require('@site/src/components/treant/WasmProvider');
          const ArcadeShell = require('@site/src/components/arcade/ArcadeShell').default;
          function Inner() {
            const { wasm, ready, error } = useWasm();
            if (error) return <div style={loadingBox}>Failed to load: {String(error)}</div>;
            if (!ready) return <div style={loadingBox}>Loading arcade…</div>;
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
