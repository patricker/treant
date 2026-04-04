import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';

type WasmModule = typeof import('mcts-wasm');

interface WasmContextType {
  wasm: WasmModule | null;
  ready: boolean;
  error: string | null;
}

const WasmContext = createContext<WasmContextType>({
  wasm: null,
  ready: false,
  error: null,
});

export function WasmProvider({ children }: { children: ReactNode }) {
  const [ctx, setCtx] = useState<WasmContextType>({
    wasm: null,
    ready: false,
    error: null,
  });

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const wasm = await import('mcts-wasm');
        await wasm.default();
        if (!cancelled) {
          setCtx({ wasm, ready: true, error: null });
        }
      } catch (e) {
        if (!cancelled) {
          setCtx({ wasm: null, ready: false, error: String(e) });
        }
      }
    })();
    return () => { cancelled = true; };
  }, []);

  return <WasmContext.Provider value={ctx}>{children}</WasmContext.Provider>;
}

export function useWasm(): WasmContextType {
  return useContext(WasmContext);
}
