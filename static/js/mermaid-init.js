// Loaded only on pages containing ```mermaid blocks.
import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11.4.1/dist/mermaid.esm.min.mjs";

const nodes = [...document.querySelectorAll("pre.mermaid")];
nodes.forEach((n) => (n.dataset.source = n.textContent));

async function render() {
  const dark = document.documentElement.dataset.theme === "dark";
  mermaid.initialize({ startOnLoad: false, securityLevel: "strict", theme: dark ? "dark" : "default" });
  nodes.forEach((n) => {
    n.removeAttribute("data-processed");
    n.textContent = n.dataset.source;
  });
  await mermaid.run({ nodes });
}

render();
document.addEventListener("themechange", render);
