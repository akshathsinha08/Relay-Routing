#!/usr/bin/env node
const base = (process.env.OLLAMA_BASE_URL ?? "http://127.0.0.1:11434").replace(/\/$/, "");

async function get(path) {
  const response = await fetch(`${base}${path}`, {
    signal: AbortSignal.timeout(10_000),
  });
  if (!response.ok) throw new Error(`${path} returned ${response.status}`);
  return response.json();
}

async function streamChat(model) {
  const response = await fetch(`${base}/api/chat`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    signal: AbortSignal.timeout(120_000),
    body: JSON.stringify({
      model,
      stream: true,
      options: { num_predict: 16 },
      messages: [{ role: "user", content: "Reply with exactly: OK" }],
    }),
  });
  if (!response.ok || !response.body) {
    throw new Error(`chat stream returned ${response.status}`);
  }
  const decoder = new TextDecoder();
  let buffer = "";
  let chunks = 0;
  let text = "";
  for await (const part of response.body) {
    buffer += decoder.decode(part, { stream: true });
    let newline = buffer.indexOf("\n");
    while (newline >= 0) {
      const line = buffer.slice(0, newline).trim();
      buffer = buffer.slice(newline + 1);
      newline = buffer.indexOf("\n");
      if (!line) continue;
      const event = JSON.parse(line);
      if (event.error) throw new Error(String(event.error));
      chunks += 1;
      if (event.message?.content) text += event.message.content;
    }
  }
  return { chunks, text: text.trim() };
}

const version = await get("/api/version");
const tags = await get("/api/tags");
const models = (tags.models ?? []).map((model) => model.name);
if (models.length === 0) {
  console.error("No Ollama models installed. Run: ollama pull qwen3.5:4b");
  process.exit(1);
}

const preferred =
  process.env.OLLAMA_MODEL ??
  models.find((name) => /qwen3\.5|qwen2\.5|llama3|gemma/i.test(name)) ??
  models[0];

console.log(`Ollama ${version.version} at ${base}`);
console.log(`Models (${models.length}): ${models.join(", ")}`);
const chat = await streamChat(preferred);
if (chat.chunks === 0) throw new Error("stream produced no events");
console.log(
  `Streaming chat OK with ${preferred}: ${chat.chunks} events, reply ${JSON.stringify(chat.text)}`,
);
