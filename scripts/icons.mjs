import { readFile, writeFile, mkdir, copyFile } from "node:fs/promises";
const catalog = JSON.parse(
  await readFile(new URL("../config/defaults.json", import.meta.url), "utf8"),
);
const names = {
  claude: "claude-color",
  codex: "openai",
  gemini: "gemini-color",
  antigravity: "antigravity-color",
  cursor: "cursor",
  copilot: "githubcopilot",
  windsurf: "windsurf",
  cline: "cline",
  roo: "roocode",
  opencode: "opencode",
  kiro: "kiro-color",
};
await mkdir(new URL("../public/icons/", import.meta.url), { recursive: true });
for (const provider of catalog.providers) {
  const destination = new URL(
    `../public/icons/${provider.id}.svg`,
    import.meta.url,
  );
  if (names[provider.id])
    await copyFile(
      new URL(
        `../node_modules/@lobehub/icons-static-svg/icons/${names[provider.id]}.svg`,
        import.meta.url,
      ),
      destination,
    );
  else {
    const label =
      { continue: "C", amazonq: "Q", aider: "A" }[provider.id] ??
      provider.name[0];
    await writeFile(
      destination,
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect x="2" y="2" width="28" height="28" rx="7" fill="none" stroke="currentColor" stroke-width="2"/><text x="16" y="22" text-anchor="middle" font-family="sans-serif" font-size="18" font-weight="700" fill="currentColor">${label}</text></svg>\n`,
    );
  }
}
console.log(`Prepared ${catalog.providers.length} offline provider icons`);
