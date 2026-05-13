import type { IncomingMessage, ServerResponse } from "node:http";
import { defineConfig, type Plugin } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

interface E2eChatMessage {
  role?: string;
  content?: string;
}

interface E2eChatRequest {
  messages?: E2eChatMessage[];
}

function readRequestBody(request: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    let body = "";
    request.setEncoding("utf8");
    request.on("data", (chunk: string) => {
      body += chunk;
    });
    request.on("end", () => resolve(body));
    request.on("error", reject);
  });
}

function e2eReplyFor(body: string): string {
  let prompt = "";
  try {
    const parsed = JSON.parse(body) as E2eChatRequest;
    const userMessages = parsed.messages?.filter((message) => message.role === "user") ?? [];
    prompt = userMessages[userMessages.length - 1]?.content?.toLowerCase() ?? "";
  } catch {
    prompt = "";
  }

  if (prompt.includes("clap")) {
    return '<anim>{"emotion":"happy","motion":"clap"}</anim>\nI am clapping happily for you.';
  }
  if (prompt.includes("angry") || prompt.includes("yell")) {
    return '<anim>{"emotion":"angry","motion":"angry"}</anim>\nI am angry and yelling with dramatic energy.';
  }
  if (prompt.includes("happy") || prompt.includes("joy")) {
    return '<anim>{"emotion":"happy","motion":"jump"}</anim>\nI am happy, excited, and full of joy.';
  }
  return "TerranSoul E2E local AI response.";
}

function sendOpenAiSse(response: ServerResponse, content: string): void {
  response.statusCode = 200;
  response.setHeader("Content-Type", "text/event-stream; charset=utf-8");
  response.setHeader("Cache-Control", "no-cache");
  response.setHeader("Connection", "keep-alive");
  response.write(`data: ${JSON.stringify({ choices: [{ delta: { content } }] })}\n\n`);
  response.write("data: [DONE]\n\n");
  response.end();
}

function sendOllamaNdjson(response: ServerResponse, content: string): void {
  response.statusCode = 200;
  response.setHeader("Content-Type", "application/x-ndjson; charset=utf-8");
  response.write(`${JSON.stringify({ message: { content }, done: false })}\n`);
  response.write(`${JSON.stringify({ done: true })}\n`);
  response.end();
}

function e2eLocalLlmPlugin(): Plugin {
  return {
    name: "terransoul-e2e-local-llm",
    configureServer(server) {
      server.middlewares.use(async (request, response, next) => {
        const url = request.url ?? "";
        const handlesLmStudio = url.startsWith("/__lmstudio/v1/chat/completions");
        const handlesOllama = url.startsWith("/__ollama/api/chat");
        if (!handlesLmStudio && !handlesOllama) {
          next();
          return;
        }

        if (request.method !== "POST") {
          response.statusCode = 405;
          response.end("Method Not Allowed");
          return;
        }

        try {
          const content = e2eReplyFor(await readRequestBody(request));
          if (handlesOllama) {
            sendOllamaNdjson(response, content);
          } else {
            sendOpenAiSse(response, content);
          }
        } catch (error) {
          response.statusCode = 500;
          response.end(String(error));
        }
      });
    },
  };
}

const e2eLocalLlmEnabled = process.env.TERRANSOUL_E2E_LOCAL_LLM === "1";

const generatedArtifactWatchIgnores = [
  "**/src-tauri/**",
  "**/target/**",
  "**/target-*/**",
];

export default defineConfig({
  plugins: [
    ...(e2eLocalLlmEnabled ? [e2eLocalLlmPlugin()] : []),
    tailwindcss(),
    vue(),
  ],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    proxy: {
      "/__ollama": {
        target: "http://localhost:11434",
        changeOrigin: true,
        rewrite: (path: string) => path.replace(/^\/__ollama/, ""),
      },
      "/__lmstudio": {
        target: "http://127.0.0.1:1234",
        changeOrigin: true,
        rewrite: (path: string) => path.replace(/^\/__lmstudio/, ""),
      },
    },
    watch: {
      ignored: generatedArtifactWatchIgnores,
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  // Pre-bundle heavy deps so Vite doesn't stall discovering their sub-modules
  // on cold start. Three.js alone has 1000+ internal files.
  optimizeDeps: {
    include: [
      "three",
      "three/examples/jsm/loaders/GLTFLoader.js",
      "three/examples/jsm/controls/OrbitControls.js",
      "@pixiv/three-vrm",
      "d3-force-3d",
    ],
  },
  build: {
    target:
      process.env.TAURI_ENV_PLATFORM == "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    // Three.js + VRM are legitimately large bundles; suppress the warning
    // rather than artificially splitting dependencies that must be co-loaded.
    chunkSizeWarningLimit: 3000,
  },
});
