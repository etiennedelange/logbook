import { defineConfig, lazyPlugins } from 'vite-plus';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig(({ mode }) => ({
  plugins: lazyPlugins(() => [react(), tailwindcss()]),
  clearScreen: false,
  // Mock mode is opened from a browser outside the dev container. Vite's `localhost` can resolve to ::1
  // alone, which a forwarded port can't reach (it just hangs), so bind IPv4 there. `tauri dev` keeps localhost.
  server: { port: 1420, strictPort: true, host: mode === 'mock' ? '127.0.0.1' : 'localhost' },
  build: { target: 'es2022', outDir: 'dist', emptyOutDir: true },
  test: { environment: 'happy-dom', setupFiles: ['./src/test/setup.ts'], css: false },
  fmt: { singleQuote: true, printWidth: 120, ignorePatterns: ['dist/**'] },
  lint: {
    jsPlugins: [{ name: 'vite-plus', specifier: 'vite-plus/oxlint-plugin' }],
    rules: { 'vite-plus/prefer-vite-plus-imports': 'error' },
    options: { typeAware: true, typeCheck: true },
  },
}));
