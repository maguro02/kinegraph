import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
    plugins: [react(), tailwindcss(), wasm(), topLevelAwait()],
    
    // WASM対応設定（vite-plugin-wasmが処理するため簡略化）
    optimizeDeps: {
        esbuildOptions: {
            target: 'esnext',
        },
    },
    
    // Worker設定
    worker: {
        format: 'es' as const,
    },
    
    // public directoryの設定
    publicDir: 'public',
    
    build: {
        target: 'esnext',
        // WASMファイルをassets/wasmディレクトリに配置
        rollupOptions: {
            output: {
                assetFileNames: (assetInfo: any) => {
                    if (assetInfo.name?.endsWith('.wasm')) {
                        return 'assets/wasm/[name]-[hash][extname]';
                    }
                    return 'assets/[name]-[hash][extname]';
                },
            },
        },
    },

    // CSS最適化設定
    css: {
        devSourcemap: true, // 開発時のソースマップ
        postcss: {
            plugins: [
                // @tailwindcss/viteプラグインが自動でTailwindCSSを処理するため、
                // PostCSS設定から除去
            ],
        },
    },

    // Vitest設定
    test: {
        environment: 'jsdom',
        globals: true,
        setupFiles: ['./src/lib/__tests__/setup.ts'],
        include: ['src/**/*.{test,spec}.{js,ts,jsx,tsx}'],
        exclude: ['node_modules', 'dist', 'src-tauri'],
        coverage: {
            reporter: ['text', 'json', 'html'],
            exclude: [
                'node_modules/',
                'src-tauri/',
                'src/lib/__tests__/',
                'dist/',
                '**/*.d.ts',
                '**/*.config.{js,ts}',
            ],
        },
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
        port: 1420,
        strictPort: true,
        host: host || false,
        hmr: host
            ? {
                  protocol: "ws",
                  host,
                  port: 1421,
              }
            : undefined,
        watch: {
            // 3. tell vite to ignore watching `src-tauri`
            ignored: ["**/src-tauri/**"],
        },
        // Cross-Origin Isolation設定（SharedArrayBuffer等を使用するWASMモジュール用）
        headers: {
            'Cross-Origin-Embedder-Policy': 'require-corp',
            'Cross-Origin-Opener-Policy': 'same-origin',
        },
        // WASMファイルの配信設定
    },
    
    // MIMEタイプの設定
    define: {
        'process.env.NODE_ENV': JSON.stringify(process.env.NODE_ENV || 'development'),
    },
}));
