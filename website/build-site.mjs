import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { marked } from "marked";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const docsRoot = path.join(repoRoot, "docs");
const buildRoot = path.join(__dirname, "build");

const docFiles = [
  "index.md",
  "getting-started.md",
  "authentication.md",
  "project-structure.md",
  "release-checklist.md",
  "trade-mainline.md",
  "api-coverage/market-data.md",
  "api-coverage/trading.md",
  "reference/index.md",
  "reference/alpaca-core.md",
  "reference/alpaca-http.md",
  "reference/alpaca-data.md",
  "reference/alpaca-trade.md",
  "reference/alpaca-mock.md",
  "reference/stocks.md",
  "reference/options-data.md",
  "reference/news.md",
  "reference/corporate-actions.md",
  "reference/account.md",
  "reference/account-configurations.md",
  "reference/activities.md",
  "reference/assets.md",
  "reference/calendar-clock.md",
  "reference/options-contracts.md",
  "reference/orders.md",
  "reference/portfolio-history.md",
  "reference/positions.md",
  "reference/watchlists.md",
];

marked.setOptions({
  gfm: true,
  breaks: false,
});

function shell(title, content) {
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>${title}</title>
  <style>
    :root { color-scheme: light; }
    body { font-family: Georgia, "Times New Roman", serif; margin: 0; background: #f5f1e8; color: #17211d; }
    header, footer { background: #163a33; color: #f5f1e8; padding: 1rem 1.5rem; }
    header a, footer a { color: #d9f4ec; text-decoration: none; }
    main { max-width: 960px; margin: 0 auto; padding: 2rem 1.5rem 4rem; }
    nav { display: flex; flex-wrap: wrap; gap: 0.75rem 1rem; margin-top: 0.75rem; }
    article { background: rgba(255,255,255,0.6); padding: 1.5rem; border-radius: 14px; box-shadow: 0 10px 30px rgba(0,0,0,0.05); }
    code, pre { font-family: "SFMono-Regular", Consolas, monospace; }
    pre { overflow-x: auto; padding: 1rem; background: #12231f; color: #f4f8f7; border-radius: 10px; }
    a { color: #0c6f5a; }
    h1, h2, h3 { color: #102823; }
  </style>
</head>
<body>
  <header>
    <strong>alpaca-rust</strong>
    <nav>
      <a href="/alpaca-rust/index.html">Home</a>
      <a href="/alpaca-rust/getting-started.html">Getting Started</a>
      <a href="/alpaca-rust/project-structure.html">Project Structure</a>
      <a href="/alpaca-rust/reference/index.html">Reference</a>
      <a href="https://docs.rs/alpaca-data">docs.rs</a>
      <a href="https://github.com/wmzhai/alpaca-rust">GitHub</a>
    </nav>
  </header>
  <main>
    <article>
      ${content}
    </article>
  </main>
  <footer>Maintained by Weiming Zhai &lt;wmzhai@gmail.com&gt;</footer>
</body>
</html>`;
}

async function ensureDir(filePath) {
  await fs.mkdir(path.dirname(filePath), { recursive: true });
}

async function buildDoc(relativePath) {
  const sourcePath = path.join(docsRoot, relativePath);
  const markdown = await fs.readFile(sourcePath, "utf8");
  const html = marked.parse(markdown);
  const outPath = path.join(buildRoot, relativePath.replace(/\.md$/, ".html"));
  await ensureDir(outPath);
  const title = relativePath.replace(/\.md$/, "");
  await fs.writeFile(outPath, shell(title, html));
}

async function buildLandingPage() {
  const landing = shell(
    "alpaca-rust",
    `
    <h1>alpaca-rust</h1>
    <p>Rust workspace for Alpaca HTTP APIs.</p>
    <ul>
      <li><a href="./getting-started.html">Getting Started</a></li>
      <li><a href="./authentication.html">Authentication</a></li>
      <li><a href="./project-structure.html">Project Structure</a></li>
      <li><a href="./reference/index.html">Reference</a></li>
      <li><a href="./release-checklist.html">Release Checklist</a></li>
    </ul>
    `
  );
  await fs.writeFile(path.join(buildRoot, "index.html"), landing);
}

await fs.rm(buildRoot, { recursive: true, force: true });
await fs.mkdir(buildRoot, { recursive: true });
for (const docFile of docFiles) {
  await buildDoc(docFile);
}
await buildLandingPage();
