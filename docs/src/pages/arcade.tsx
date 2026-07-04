import type { JSX } from 'react';
import type React from 'react';
import Head from '@docusaurus/Head';
import ErrorBoundary from '@docusaurus/ErrorBoundary';
import BrowserOnly from '@docusaurus/BrowserOnly';

// The arcade deliberately does NOT use @theme/Layout. The docs navbar and
// footer must never exist on this route — earlier revisions rendered them and
// hid them with CSS/JS, which leaked chrome during hydration, on slow loads
// and through stale service-worker shells. No Layout → nothing to hide, in
// any loading state. (No arcade component uses Layout's theme context; the
// arcade carries its own accent theming.)

// The arcade is always the dark CRT skin, but this page's loading fallback
// renders before the skin mounts — on the default light site theme that meant
// a white flash on first load. Paint the placeholder dark from the first frame.
const loadingBox: React.CSSProperties = {
  minHeight: '100vh',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  background: '#05060a',
  color: '#8ce8a9',
  fontFamily: 'monospace',
  fontSize: '1rem',
};

function ArcadeError({ error, tryAgain }: { error: Error; tryAgain: () => void }) {
  return (
    <div style={loadingBox}>
      <div style={{ textAlign: 'center' }}>
        <p>The arcade hit an error: {String(error.message)}</p>
        <button onClick={tryAgain} style={{ padding: '8px 16px', cursor: 'pointer' }}>
          Try again
        </button>
      </div>
    </div>
  );
}

export default function ArcadePage(): JSX.Element {
  return (
    <>
      <Head>
        <title>Arcade | Treant</title>
        <meta name="description" content="Pass-and-play family games powered by treant MCTS" />
        {/* Backdrop + navbar-height reset in the served HTML from byte one
            (a <link>, not an inline <style>: Docusaurus SSG emits helmet link
            tags but drops style tags). react-helmet removes it again when
            navigating to a docs route. */}
        <link rel="stylesheet" href="/arcade-shell.css" />
      </Head>
      <ErrorBoundary fallback={ArcadeError}>
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
      </ErrorBoundary>
    </>
  );
}
