// Site-wide ASCII background: three cursor-driven simulations (ripple / life /
// field) computed in a tiny WebAssembly module, rendered into a fixed <pre>
// behind every page. The footer switch picks one or turns it off; the choice
// is remembered in localStorage (read early by an inline script in <head>).
(function () {
  "use strict";
  var root = document.documentElement;
  var pre = document.getElementById("fx-screen");
  if (!pre) return;
  var stat = document.getElementById("fx-stat");
  var btns = document.querySelectorAll(".fx-switch button");
  var rootStyle = root.style;
  var landing = document.body.classList.contains("landing");
  var reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
  var touch = matchMedia("(hover: none)").matches;
  // Reading pages don't need 60 fps behind the text.
  var FRAME_MS = landing ? 0 : 1000 / 24;

  var MODES = ["ripple", "life", "field", "off"];
  var mode = MODES.indexOf(root.dataset.fx) >= 0 ? root.dataset.fx : "ripple";
  function pressed() {
    for (var i = 0; i < btns.length; i++) {
      btns[i].setAttribute("aria-pressed", btns[i].getAttribute("data-key") === mode ? "true" : "false");
    }
  }
  pressed();

  var engine = null, loading = null;
  function setStat(t) { if (stat) stat.textContent = t; }

  function choose(key) {
    if (MODES.indexOf(key) < 0) return;
    mode = key;
    root.dataset.fx = key;
    try { localStorage.setItem("fx", key); } catch (e) {}
    pressed();
    if (key === "off") { if (engine) engine.stop(); setStat("bg off"); return; }
    boot().then(function (en) { if (en) en.start(key); });
  }
  for (var i = 0; i < btns.length; i++) {
    (function (b) { b.addEventListener("click", function () { choose(b.getAttribute("data-key")); }); })(btns[i]);
  }

  // ── Pointer → grid coordinates (the grid may be larger than the viewport) ──
  var mx = -1, my = -1, held = 0, clickEdge = 0;
  function at(cx, cy) {
    var r = pre.getBoundingClientRect();
    var fx = (cx - r.left) / r.width, fy = (cy - r.top) / r.height;
    if (fx < 0 || fx > 1 || fy < 0 || fy > 1) { mx = -1; my = -1; return; }
    mx = fx; my = fy;
  }
  var CONTROL = "a,button,input,textarea,select,summary,label,dialog,pre,code,[role=button]";
  addEventListener("pointermove", function (e) { at(e.clientX, e.clientY); }, { passive: true });
  addEventListener("pointerdown", function (e) {
    at(e.clientX, e.clientY);
    // Clicking a link or a button shouldn't also splash the background.
    if (e.target.closest && e.target.closest(CONTROL)) return;
    held = 1; clickEdge = 1;
  }, { passive: true });
  addEventListener("pointerup", function () { held = 0; }, { passive: true });
  addEventListener("pointercancel", function () { held = 0; mx = -1; my = -1; }, { passive: true });
  document.addEventListener("pointerleave", function () { mx = -1; my = -1; held = 0; });

  if (mode === "off") setStat("bg off");
  else boot().then(function (en) { if (en) en.start(mode); });

  function boot() {
    if (!loading) loading = !("WebAssembly" in window) ? Promise.resolve(null) : fetch("/wasm/sims.wasm")
      .then(function (r) { return r.arrayBuffer(); })
      .then(function (b) { return WebAssembly.instantiate(b, {}); })
      .then(function (res) { engine = makeEngine(res.instance.exports); return engine; })
      .catch(function (e) { setStat("bg unavailable: " + e.message); return null; });
    return loading;
  }

  function makeEngine(ex) {
    var GW = ex.width(), GH = ex.height();
    var out = new Uint8Array(ex.memory.buffer, ex.chars(), ex.out_len());
    var par = new Float32Array(ex.memory.buffer, ex.params(), 16);
    var dec = new TextDecoder("ascii");

    // The grid is a fixed GW×GH. Stretching it to the viewport squashes the
    // glyphs on portrait phones (≈2px wide), so instead "cover" the screen:
    // keep the cell aspect within a sane range, enforce a minimum glyph
    // width, and centre the (possibly larger) grid, cropping the overflow.
    var cellW = 1, cellH = 1, lastW = 0, lastH = 0;
    var MIN_ASPECT = 1.0, MAX_ASPECT = 2.0, MIN_CELL_W = 4.5;
    function sizePre() {
      var w = window.innerWidth, h = window.innerHeight;
      // Mobile browsers fire resize when the URL bar slides; ignore small
      // height-only changes so the animation doesn't jump.
      if (w === lastW && Math.abs(h - lastH) < 120 && lastH) return;
      lastW = w; lastH = h;
      var probe = document.createElement("canvas").getContext("2d");
      probe.font = "100px " + getComputedStyle(pre).fontFamily;
      var ratio = probe.measureText("0000000000").width / 1000; // advance per px
      cellW = w / GW; cellH = h / GH;
      if (cellH / cellW > MAX_ASPECT) cellW = cellH / MAX_ASPECT;
      if (cellH / cellW < MIN_ASPECT) cellH = cellW * MIN_ASPECT;
      if (cellW < MIN_CELL_W) { var k = MIN_CELL_W / cellW; cellW *= k; cellH *= k; }
      var W = GW * cellW, H = GH * cellH;
      pre.style.width = W + "px";
      pre.style.height = H + "px";
      pre.style.left = ((w - W) / 2) + "px";
      pre.style.top = ((h - H) / 2) + "px";
      pre.style.fontSize = (cellW / ratio) + "px";
      pre.style.lineHeight = cellH + "px";
    }
    (document.fonts ? document.fonts.ready : Promise.resolve()).then(function () { lastW = 0; sizePre(); });
    sizePre();
    addEventListener("resize", sizePre);

    // On the landing, touch drags drive the simulation while the page fits on
    // screen; once it overflows (short phones, landscape) keep scrolling.
    function touchMode() {
      if (!landing) return;
      var fits = root.scrollHeight <= window.innerHeight + 1;
      document.body.style.touchAction = fits && mode !== "off" ? "none" : "";
    }
    addEventListener("resize", touchMode);

    function cell() { return [mx < 0 ? -1 : (mx * GW) | 0, my < 0 ? -1 : (my * GH) | 0]; }

    var charges = [
      { x: 50, y: 34, vx: 0.07, vy: 0.05, q: 7 },
      { x: 150, y: 34, vx: -0.06, vy: 0.05, q: -7 },
      { x: 64, y: 86, vx: 0.05, vy: -0.06, q: -6 },
      { x: 150, y: 86, vx: -0.05, vy: -0.05, q: 6 }
    ];
    var spin = 1;

    var sims = {
      ripple: {
        phosphor: "#5f8378", accent: "#b7935a", opacity: 0.55,
        light: { phosphor: "#4f7468", accent: "#8d5a17", opacity: 0.42 },
        hint: touch ? "drag to ripple · tap to splash" : "move to ripple · click to splash",
        enter: function () { ex.clear(); },
        frame: function () { var c = cell(); ex.step_ripple(c[0], c[1], held); }
      },
      life: {
        phosphor: "#b89155", accent: "#6f9f8f", opacity: 0.6,
        light: { phosphor: "#9a7334", accent: "#3f6b5f", opacity: 0.45 },
        hint: touch ? "drag to draw · tap drops a glider" : "draw cells · click drops a glider",
        enter: function () { ex.clear(); ex.seed_life(130); this._t = 0; },
        frame: function () { var c = cell(); var gen = (this._t++ % 5 === 0) ? 1 : 0; ex.step_life(c[0], c[1], clickEdge, gen); }
      },
      field: {
        phosphor: "#808f99", accent: "#c0633f", opacity: 0.62,
        light: { phosphor: "#66737c", accent: "#a4502f", opacity: 0.45 },
        hint: touch ? "drag to stir · tap reverses spin" : "stir the field · click reverses spin",
        enter: function () { ex.clear(); },
        frame: function () {
          if (clickEdge) spin = -spin;
          for (var i = 0; i < charges.length; i++) {
            var c = charges[i];
            c.x += c.vx; c.y += c.vy;
            if (c.x < 6 || c.x > GW - 6) c.vx *= -1;
            if (c.y < 6 || c.y > GH - 6) c.vy *= -1;
          }
          par[0] = cellH / cellW; par[1] = spin; par[2] = charges.length;
          for (var j = 0; j < charges.length; j++) {
            par[3 + j * 3] = charges[j].x; par[4 + j * 3] = charges[j].y; par[5 + j * 3] = charges[j].q;
          }
          var cc = cell(); ex.step_field(cc[0], cc[1]);
        }
      }
    };

    var cur = null;
    // Colours follow the site theme (paper vs near-black).
    function paint() {
      if (!cur) return;
      var s = sims[cur];
      var c = root.dataset.theme === "dark" ? s : s.light;
      rootStyle.setProperty("--fx-phosphor", c.phosphor);
      rootStyle.setProperty("--fx-accent", c.accent);
      rootStyle.setProperty("--fx-opacity", c.opacity);
    }
    document.addEventListener("themechange", paint);

    function draw() { sims[cur].frame(); clickEdge = 0; pre.textContent = dec.decode(out); }

    var running = false, frames = 0, acc = 0, last = 0, prev = 0;
    function loop(t) {
      if (!running) return;
      requestAnimationFrame(loop);
      if (FRAME_MS && t - prev < FRAME_MS - 1) return;
      prev = t;
      draw();
      frames++; acc += t - last; last = t;
      if (acc >= 600) {
        setStat(cur + " · " + GW + "×" + GH + " · ~" + Math.round(frames * 1000 / acc) + " fps · " + sims[cur].hint);
        frames = 0; acc = 0;
      }
    }
    function run() {
      if (running || document.hidden || mode === "off") return;
      running = true; last = prev = performance.now(); frames = 0; acc = 0;
      requestAnimationFrame(loop);
    }
    function stop() { running = false; }
    document.addEventListener("visibilitychange", function () { if (document.hidden) stop(); else if (!reduce) run(); });

    // Reduced motion: no loop, only redraw in response to input.
    var queued = false;
    function tick() {
      if (!reduce || mode === "off" || queued) return;
      queued = true;
      requestAnimationFrame(function () { queued = false; draw(); });
    }
    addEventListener("pointermove", tick, { passive: true });
    addEventListener("pointerdown", tick, { passive: true });

    return {
      start: function (key) {
        cur = key;
        paint();
        sims[key].enter();
        touchMode();
        if (reduce) {
          draw();
          setStat(cur + " · " + GW + "×" + GH + " · paused · " + (touch ? "drag" : "move") + " to animate");
        } else {
          run();
        }
      },
      stop: function () { stop(); touchMode(); }
    };
  }
})();
