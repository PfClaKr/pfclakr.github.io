// Loaded only on pages containing ```mermaid blocks.
import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11.4.1/dist/mermaid.esm.min.mjs";

const nodes = [...document.querySelectorAll("pre.mermaid")];
nodes.forEach((n) => (n.dataset.source = n.textContent));

// Phones: a left-to-right flowchart is often far too wide for the column, so
// also try it top-to-bottom and keep whichever orientation is narrower.
const narrow = matchMedia("(max-width: 700px)");
const HORIZONTAL = /^(\s*(?:flowchart|graph))\s+(?:LR|RL)\b/m;
// Never shrink a diagram below this fraction of its natural size; scroll instead.
const MIN_SCALE = 0.7;
let generation = 0;

function viewBoxWidth(svg) {
  const m = svg.match(/viewBox="[-\d.]+\s+[-\d.]+\s+([\d.]+)/);
  return m ? parseFloat(m[1]) : Infinity;
}

async function draw(node, index, gen) {
  const base = node.dataset.source;
  const candidates = [base];
  if (narrow.matches && HORIZONTAL.test(base)) candidates.push(base.replace(HORIZONTAL, "$1 TD"));
  let best = null;
  for (let k = 0; k < candidates.length; k++) {
    try {
      const out = await mermaid.render(`mmd-${gen}-${index}-${k}`, candidates[k]);
      const width = viewBoxWidth(out.svg);
      if (!best || width < best.width) best = { ...out, width };
    } catch (e) {
      if (k === 0) {
        // Keep the source readable instead of an error graphic.
        node.textContent = base;
        console.warn("mermaid:", e);
        return;
      }
    }
  }
  if (gen !== generation) return; // a newer render (theme/orientation change) won
  node.innerHTML = best.svg;
  if (best.bindFunctions) best.bindFunctions(node);
  node.setAttribute("data-processed", "true");
}

function fit() {
  for (const n of nodes) {
    const svg = n.querySelector("svg");
    if (!svg || !svg.viewBox || !svg.viewBox.baseVal.width) continue;
    const natural = svg.viewBox.baseVal.width;
    const style = getComputedStyle(n);
    const avail = n.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
    const wide = natural * MIN_SCALE > avail;
    svg.style.maxWidth = wide ? "none" : `${natural}px`;
    svg.style.width = wide ? `${Math.round(natural * MIN_SCALE)}px` : "100%";
    n.classList.toggle("is-wide", wide);
  }
}

async function render() {
  const gen = ++generation;
  const dark = document.documentElement.dataset.theme === "dark";
  mermaid.initialize({ startOnLoad: false, securityLevel: "strict", theme: dark ? "dark" : "default" });
  for (let i = 0; i < nodes.length; i++) await draw(nodes[i], i, gen);
  if (gen === generation) fit();
}

render();
document.addEventListener("themechange", render);
narrow.addEventListener("change", render);
let resizeTimer;
addEventListener("resize", () => {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(fit, 150);
});
