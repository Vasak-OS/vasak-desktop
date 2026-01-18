// file: vite.config.ts (Mejorado)
import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import path from "node:path";

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
    // Evita duplicados de módulos
    dedupe: ["vue", "pinia"],
  },

  // Optimización de build
  build: {
    target: "es2020",
    minify: "terser",
    terserOptions: {
      compress: {
        drop_console: true,
        drop_debugger: true,
        dead_code: true,
      },
      format: {
        comments: false,
      },
    },
    // Configuración de rolling-up para code splitting automático
    rollupOptions: {
      output: {
        // Dividir dependencias en chunks separados
        manualChunks: {
          vendor: ["vue", "vue-router", "pinia"],
          tauri: [
            "@tauri-apps/api",
            "@tauri-apps/plugin-fs",
            "@tauri-apps/plugin-shell",
            "@tauri-apps/plugin-opener",
          ],
          ui: ["@vasakgroup/vue-libvasak"],
          vasak: [
            "@vasakgroup/plugin-config-manager",
            "@vasakgroup/plugin-network-manager",
            "@vasakgroup/plugin-bluetooth-manager",
            "@vasakgroup/plugin-user-data",
          ],
        },
      },
    },
    // Limite de advertencia de chunk size
    chunkSizeWarningLimit: 1000,
    // Generar source maps en desarrollo
    sourcemap: process.env.NODE_ENV === "development",
  },

  // Optimización de server en desarrollo
  server: {
    fs: {
      allow: [".."],
    },
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
      // Ignorar cambios en src-tauri
      ignored: ["**/src-tauri/**"],
      // Debounce para cambios de archivo
      awaitWriteFinish: {
        stabilityThreshold: 100,
        pollInterval: 100,
      },
    },
    // Pre-transformación de requests para optimizar HMR
    preTransformRequests: true,
    // No usar modo middleware
    middlewareMode: false,
  },

  // Configuración de CSS
  css: {
    preprocessorOptions: {
      // Opciones si se usa SCSS
    },
  },

  // Optimización de dependencias
  optimizeDeps: {
    include: [
      "vue",
      "vue-router",
      "pinia",
      "@tauri-apps/api",
      "@vasakgroup/vue-libvasak",
    ],
    // Excluir dependencias que manejan sus propios módulos
    exclude: ["@tauri-apps/plugin-shell"],
  },

  // Environment variables
  define: {
    "process.env.VITE_APP_VERSION": JSON.stringify(process.env.npm_package_version),
  },
});
