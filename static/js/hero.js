// Interactive ASCII hero: three cursor-driven simulations (ripple / life / field)
// computed in a tiny WebAssembly module. Scoped to #hero-fx; no-ops elsewhere.
(function () {
  "use strict";
  var box = document.getElementById("hero-fx");
  if (!box || !("WebAssembly" in window)) return;
  var pre = document.getElementById("hero-screen");
  var stat = document.getElementById("hero-stat");
  var rootStyle = document.documentElement.style;
  var reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
  var touch = matchMedia("(hover: none)").matches;

  var mx = -1, my = -1, held = 0, clickEdge = 0;
  // Map pointer coordinates onto the ASCII grid (which may be larger than the
  // viewport and centred, so measure the <pre> itself rather than the overlay).
  function at(cx, cy) {
    var r = pre.getBoundingClientRect();
    var fx = (cx - r.left) / r.width, fy = (cy - r.top) / r.height;
    if (fx < 0 || fx > 1 || fy < 0 || fy > 1) { mx = -1; my = -1; return; }
    mx = fx; my = fy;
  }
  box.addEventListener("pointermove", function (e) { at(e.clientX, e.clientY); }, { passive: true });
  box.addEventListener("pointerdown", function (e) { held = 1; clickEdge = 1; at(e.clientX, e.clientY); }, { passive: true });
  window.addEventListener("pointerup", function () { held = 0; }, { passive: true });
  box.addEventListener("pointerleave", function () { mx = -1; my = -1; held = 0; });

  fetch("/wasm/sims.wasm")
    .then(function (r) { return r.arrayBuffer(); })
    .then(function (b) { return WebAssembly.instantiate(b, {}); })
    .then(function (res) {
      var ex = res.instance.exports;
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
      // When the landing fits on screen, let touch drags drive the simulation;
      // when it overflows (short phones, landscape) keep vertical scrolling.
      function touchMode() {
        var fits = document.documentElement.scrollHeight <= window.innerHeight + 1;
        box.style.touchAction = fits ? "none" : "pan-y";
      }
      touchMode();
      addEventListener("resize", touchMode);
      (document.fonts ? document.fonts.ready : Promise.resolve()).then(function () { lastW = 0; sizePre(); });
      sizePre();
      addEventListener("resize", sizePre);

      function cell() { return [mx < 0 ? -1 : (mx * GW) | 0, my < 0 ? -1 : (my * GH) | 0]; }

      var charges = [
        { x: 50, y: 34, vx: 0.07, vy: 0.05, q: 7 },
        { x: 150, y: 34, vx: -0.06, vy: 0.05, q: -7 },
        { x: 64, y: 86, vx: 0.05, vy: -0.06, q: -6 },
        { x: 150, y: 86, vx: -0.05, vy: -0.05, q: 6 }
      ];
      var spin = 1;

      var themes = {
        ripple: {
          phosphor: "#5f8378", accent: "#b7935a", opacity: 0.55,
          hint: touch ? "drag to ripple · tap to splash" : "move to ripple · click to splash",
          enter: function () { ex.clear(); },
          frame: function () { var c = cell(); ex.step_ripple(c[0], c[1], held); }
        },
        life: {
          phosphor: "#b89155", accent: "#6f9f8f", opacity: 0.6,
          hint: touch ? "drag to draw · tap drops a glider" : "draw cells · click drops a glider",
          enter: function () { ex.clear(); ex.seed_life(130); this._t = 0; },
          frame: function () { var c = cell(); var gen = (this._t++ % 5 === 0) ? 1 : 0; ex.step_life(c[0], c[1], clickEdge, gen); }
        },
        field: {
          phosphor: "#808f99", accent: "#c0633f", opacity: 0.62,
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

      var cur = "ripple";
      function apply(key) {
        cur = key;
        var th = themes[key];
        rootStyle.setProperty("--fx-phosphor", th.phosphor);
        rootStyle.setProperty("--fx-accent", th.accent);
        rootStyle.setProperty("--fx-opacity", th.opacity);
        th.enter();
        var btns = box.querySelectorAll(".fx-switch button");
        for (var i = 0; i < btns.length; i++) {
          btns[i].setAttribute("aria-pressed", btns[i].getAttribute("data-key") === key ? "true" : "false");
        }
      }
      var btns = box.querySelectorAll(".fx-switch button");
      for (var i = 0; i < btns.length; i++) {
        (function (b) { b.addEventListener("click", function () { apply(b.getAttribute("data-key")); }); })(btns[i]);
      }

      var frames = 0, acc = 0, last = performance.now(), fps = 0, visible = true, running = false;
      function loop(t) {
        if (!visible) { running = false; return; }
        themes[cur].frame();
        clickEdge = 0;
        pre.textContent = dec.decode(out);
        frames++; acc += t - last; last = t;
        if (acc >= 600) {
          fps = Math.round(frames * 1000 / acc); frames = 0; acc = 0;
          if (stat) stat.textContent = cur + " · " + GW + "×" + GH + " · ~" + fps + " fps · " + themes[cur].hint;
        }
        requestAnimationFrame(loop);
      }
      function startLoop() { if (!running) { running = true; last = performance.now(); requestAnimationFrame(loop); } }

      apply("ripple");

      if (reduce) {
        if (stat) stat.textContent = cur + " · " + GW + "×" + GH + " · paused · " + (touch ? "drag" : "move") + " to animate";
        var queued = false;
        function tick() {
          if (queued) return; queued = true;
          requestAnimationFrame(function () { queued = false; themes[cur].frame(); clickEdge = 0; pre.textContent = dec.decode(out); });
        }
        box.addEventListener("pointermove", tick, { passive: true });
        box.addEventListener("pointerdown", tick, { passive: true });
        for (var k = 0; k < btns.length; k++) btns[k].addEventListener("click", tick);
        themes[cur].frame(); pre.textContent = dec.decode(out);
        return;
      }

      // pause the loop when the hero scrolls out of view
      if ("IntersectionObserver" in window) {
        new IntersectionObserver(function (es) {
          visible = es[0].isIntersecting;
          if (visible) startLoop();
        }, { threshold: 0.01 }).observe(box);
      }
      visible = true; startLoop();
    })
    .catch(function (e) { if (stat) stat.textContent = "init failed: " + e.message; });
})();
