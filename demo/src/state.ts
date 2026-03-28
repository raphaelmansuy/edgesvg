/** Reactive state store using EventTarget for UI updates. */

import type {
  AnalyzeResponse,
  OutputTab,
  ProcessStatus,
  QualityPreset,
  VectorizeMethod,
  VectorizeResponse,
  WasmStatus,
} from './types';

export interface AppState {
  imageBytes: Uint8Array | null;
  imageUrl: string | null;
  fileName: string;

  // Vectorization parameters
  method: VectorizeMethod;
  quality: QualityPreset;
  targetSsim: number;
  maxFileSize: number;
  maxIterations: number;
  colors: number | null;

  // Results
  result: VectorizeResponse | null;
  analysis: AnalyzeResponse | null;

  // UI state
  wasmStatus: WasmStatus;
  processStatus: ProcessStatus;
  activeTab: OutputTab;
  errorMessage: string | null;
  darkMode: boolean;
}

type StateKey = keyof AppState;

class StateStore extends EventTarget {
  private state: AppState;

  constructor() {
    super();
    this.state = {
      imageBytes: null,
      imageUrl: null,
      fileName: '',

      method: 'auto',
      quality: 'ultra',
      targetSsim: 0.998,
      maxFileSize: 100_000,
      maxIterations: 4,
      colors: null,

      result: null,
      analysis: null,

      wasmStatus: 'idle',
      processStatus: 'idle',
      activeTab: 'svg',
      errorMessage: null,
      darkMode: false,
    };
  }

  get<K extends StateKey>(key: K): AppState[K] {
    return this.state[key];
  }

  set<K extends StateKey>(key: K, value: AppState[K]): void {
    this.state[key] = value;
    this.dispatchEvent(new CustomEvent('change', { detail: { key, value } }));
  }

  subscribe(key: StateKey, callback: (value: unknown) => void): () => void {
    const handler = (e: Event) => {
      const { key: k, value } = (e as CustomEvent).detail;
      if (k === key) callback(value);
    };
    this.addEventListener('change', handler);
    return () => this.removeEventListener('change', handler);
  }
}

export const store = new StateStore();
